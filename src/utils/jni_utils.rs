use blocking::unblock;
use std::collections::HashMap;

use jni::{
    JNIEnv, JavaVM,
    objects::{JClass, JObject, JValue},
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{LazyLock, Mutex};

type LocationUpdatesCallback = Box<dyn Fn((f64, f64, f32)) + Send + Sync>;
type LocationEnabledCallback = Box<dyn Fn(bool) + Send + Sync>;

static PERMISSION_REQUEST_ID: AtomicI64 = AtomicI64::new(1);
static PERMISSION_REQUESTS: LazyLock<Mutex<HashMap<i64, std::sync::mpsc::Sender<bool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn clear_pending_java_exception(env: &mut JNIEnv) {
    if let Ok(true) = env.exception_check() {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
}

pub fn get_cache_dir_android() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast())? };

    let mut env = vm.attach_current_thread()?;

    let file_obj = env
        .call_method(
            unsafe { JObject::from_raw(ctx.context().cast()) },
            "getCacheDir",
            "()Ljava/io/File;",
            &[],
        )?
        .l()?;

    let absolute_path_obj = env
        .call_method(file_obj, "getAbsolutePath", "()Ljava/lang/String;", &[])?
        .l()?;

    let absolute_path: String = env.get_string(&absolute_path_obj.into())?.into();

    Ok(PathBuf::from(absolute_path))
}

pub fn get_bar_sizes() -> Result<(f32, f32, f32, f32), Box<dyn std::error::Error>> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast())? };

    let mut env = vm.attach_current_thread()?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let helper_class = match (|| -> Result<JClass, jni::errors::Error> {
        let class_loader = env
            .call_method(&context, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?
            .l()?;
        let helper_class_name = env.new_string("dev.drigster.taskupeatus.AndroidUiHelper")?;
        let helper_class_obj = env
            .call_method(
                &class_loader,
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                &[JValue::Object(&helper_class_name)],
            )?
            .l()?;
        Ok(JClass::from(helper_class_obj))
    })() {
        Ok(class) => class,
        Err(err) => {
            println!("[Print] Error finding AndroidUiHelper: {:?}", err);
            clear_pending_java_exception(&mut env);
            return Ok((0.0, 0.0, 0.0, 0.0));
        }
    };

    let insets_array = match env.call_static_method(
        helper_class,
        "getBarSizes",
        "(Landroid/content/Context;)[F",
        &[JValue::Object(&context)],
    ) {
        Ok(value) => value.l()?,
        Err(err) => {
            println!("[Print] Error calling getBarSizes: {:?}", err);
            clear_pending_java_exception(&mut env);
            return Ok((0.0, 0.0, 0.0, 0.0));
        }
    };

    if insets_array.is_null() {
        return Ok((0.0, 0.0, 0.0, 0.0));
    }

    let insets_array = jni::objects::JFloatArray::from(insets_array);
    let len = env.get_array_length(&insets_array)?;
    if len < 4 {
        return Ok((0.0, 0.0, 0.0, 0.0));
    }

    let mut values = [0.0_f32; 4];
    env.get_float_array_region(&insets_array, 0, &mut values)?;

    Ok((values[0], values[1], values[2], values[3]))
}

pub async fn check_and_request_permissions() -> Result<bool, Box<dyn std::error::Error>> {
    let request_id = PERMISSION_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    let (tx, rx) = std::sync::mpsc::channel::<bool>();

    {
        let mut requests = PERMISSION_REQUESTS
            .lock()
            .map_err(|_| "Permission requests lock poisoned")?;
        requests.insert(request_id, tx);
    }

    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast())? };
    let mut env = vm.attach_current_thread()?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let setup_result: Result<(), jni::errors::Error> = (|| {
        let class_loader = env
            .call_method(&context, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?
            .l()?;
        let helper_class_name = env.new_string("dev.drigster.taskupeatus.RustPermissionHelper")?;
        let helper_class_obj = env
            .call_method(
                &class_loader,
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                &[JValue::Object(&helper_class_name)],
            )?
            .l()?;
        let helper_class = JClass::from(helper_class_obj);

        env.call_static_method(
            helper_class,
            "requestLocationPermission",
            "(Landroid/content/Context;J)V",
            &[JValue::Object(&context), JValue::Long(request_id)],
        )?;

        Ok(())
    })();

    if let Err(err) = setup_result {
        clear_pending_java_exception(&mut env);
        if let Ok(mut requests) = PERMISSION_REQUESTS.lock() {
            requests.remove(&request_id);
        }
        return Err(Box::new(err));
    }

    let permission_result =
        unblock(
            move || match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                Ok(granted) => granted,
                Err(_) => {
                    if let Ok(mut requests) = PERMISSION_REQUESTS.lock() {
                        requests.remove(&request_id);
                    }
                    false
                }
            },
        )
        .await;

    Ok(permission_result)
}

pub fn get_last_known_location() -> Result<(f64, f64), Box<dyn std::error::Error>> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast())? };
    let mut env = vm.attach_current_thread()?;

    let location_service = env.new_string("location")?;
    let location_manager = env
        .call_method(
            unsafe { JObject::from_raw(ctx.context().cast()) },
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&location_service)],
        )?
        .l()?;

    // Get last known location from GPS provider
    let provider = env.new_string("gps")?;
    let location = env
        .call_method(
            &location_manager,
            "getLastKnownLocation",
            "(Ljava/lang/String;)Landroid/location/Location;",
            &[JValue::Object(&provider)],
        )?
        .l()?;

    if location.is_null() {
        // Try network provider as fallback
        let network_provider = env.new_string("network")?;
        let network_location = env
            .call_method(
                &location_manager,
                "getLastKnownLocation",
                "(Ljava/lang/String;)Landroid/location/Location;",
                &[JValue::Object(&network_provider)],
            )?
            .l()?;

        if network_location.is_null() {
            return Err("No location available".into());
        }

        let lat = env
            .call_method(&network_location, "getLatitude", "()D", &[])?
            .d()?;
        let lng = env
            .call_method(&network_location, "getLongitude", "()D", &[])?
            .d()?;
        return Ok((lat, lng));
    }

    let lat = env.call_method(&location, "getLatitude", "()D", &[])?.d()?;
    let lng = env
        .call_method(&location, "getLongitude", "()D", &[])?
        .d()?;
    Ok((lat, lng))
}

pub fn start_location_enabled_updates<F>(callback: F) -> Result<i64, Box<dyn std::error::Error>>
where
    F: Fn(bool) + Send + Sync + 'static,
{
    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast())? };
    let mut env = vm.attach_current_thread()?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let callback_box: Box<LocationEnabledCallback> = Box::new(Box::new(callback));
    let callback_ptr = Box::into_raw(callback_box) as i64;

    let setup_result: Result<(), jni::errors::Error> = (|| {
        let class_loader = env
            .call_method(&context, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?
            .l()?;
        let helper_class_name = env.new_string("dev.drigster.taskupeatus.RustLocationHelper")?;
        let helper_class_obj = env
            .call_method(
                &class_loader,
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                &[JValue::Object(&helper_class_name)],
            )?
            .l()?;
        let helper_class = JClass::from(helper_class_obj);

        env.call_static_method(
            helper_class,
            "startLocationEnabledUpdates",
            "(Landroid/content/Context;J)V",
            &[JValue::Object(&context), JValue::Long(callback_ptr)],
        )?;

        Ok(())
    })();

    if let Err(err) = setup_result {
        clear_pending_java_exception(&mut env);
        unsafe {
            drop(Box::from_raw(callback_ptr as *mut LocationEnabledCallback));
        }
        return Err(Box::new(err));
    }

    Ok(callback_ptr)
}

// pub fn stop_location_enabled_updates(callback_ptr: i64) -> Result<(), Box<dyn std::error::Error>> {
//     let ctx = ndk_context::android_context();
//     let vm = unsafe { JavaVM::from_raw(ctx.vm().cast())? };
//     let mut env = vm.attach_current_thread()?;
//     let context = unsafe { JObject::from_raw(ctx.context().cast()) };

//     let class_loader = env
//         .call_method(&context, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?
//         .l()?;
//     let helper_class_name = env.new_string("dev.drigster.taskupeatus.RustLocationHelper")?;
//     let helper_class_obj = env
//         .call_method(
//             &class_loader,
//             "loadClass",
//             "(Ljava/lang/String;)Ljava/lang/Class;",
//             &[JValue::Object(&helper_class_name)],
//         )?
//         .l()?;
//     let helper_class = JClass::from(helper_class_obj);

//     env.call_static_method(
//         helper_class,
//         "stopLocationEnabledUpdates",
//         "(Landroid/content/Context;J)V",
//         &[JValue::Object(&context), JValue::Long(callback_ptr)],
//     )?;

//     unsafe {
//         drop(Box::from_raw(callback_ptr as *mut LocationEnabledCallback));
//     }

//     Ok(())
// }

pub fn start_location_updates<F>(callback: F) -> Result<i64, Box<dyn std::error::Error>>
where
    F: Fn((f64, f64, f32)) + Send + Sync + 'static,
{
    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast())? };
    let mut env = vm.attach_current_thread()?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let callback_box: Box<LocationUpdatesCallback> = Box::new(Box::new(callback));
    let callback_ptr = Box::into_raw(callback_box) as i64;

    let setup_result: Result<(), jni::errors::Error> = (|| {
        let class_loader = env
            .call_method(&context, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?
            .l()?;
        let helper_class_name = env.new_string("dev.drigster.taskupeatus.RustLocationHelper")?;
        let helper_class_obj = env
            .call_method(
                &class_loader,
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                &[JValue::Object(&helper_class_name)],
            )?
            .l()?;
        let helper_class = JClass::from(helper_class_obj);

        env.call_static_method(
            helper_class,
            "startLocationUpdates",
            "(Landroid/content/Context;J)V",
            &[JValue::Object(&context), JValue::Long(callback_ptr)],
        )?;

        Ok(())
    })();

    if let Err(err) = setup_result {
        clear_pending_java_exception(&mut env);
        unsafe {
            drop(Box::from_raw(callback_ptr as *mut LocationUpdatesCallback));
        }
        return Err(Box::new(err));
    }

    Ok(callback_ptr)
}

// pub fn stop_location_updates(callback_ptr: i64) -> Result<(), Box<dyn std::error::Error>> {
//     let ctx = ndk_context::android_context();
//     let vm = unsafe { JavaVM::from_raw(ctx.vm().cast())? };
//     let mut env = vm.attach_current_thread()?;
//     let context = unsafe { JObject::from_raw(ctx.context().cast()) };

//     let class_loader = env
//         .call_method(&context, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?
//         .l()?;
//     let helper_class_name = env.new_string("dev.drigster.taskupeatus.RustLocationHelper")?;
//     let helper_class_obj = env
//         .call_method(
//             &class_loader,
//             "loadClass",
//             "(Ljava/lang/String;)Ljava/lang/Class;",
//             &[JValue::Object(&helper_class_name)],
//         )?
//         .l()?;
//     let helper_class = JClass::from(helper_class_obj);

//     env.call_static_method(
//         helper_class,
//         "stopLocationUpdates",
//         "(Landroid/content/Context;J)V",
//         &[JValue::Object(&context), JValue::Long(callback_ptr)],
//     )?;

//     unsafe {
//         drop(Box::from_raw(callback_ptr as *mut LocationUpdatesCallback));
//     }

//     Ok(())
// }

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_drigster_taskupeatus_RustLocationHelper_onLocationResult(
    _env: JNIEnv,
    _class: jni::objects::JClass,
    callback_ptr: i64,
    lat: f64,
    lng: f64,
    accuracy: f32,
) {
    unsafe {
        let callback: Box<Box<dyn FnOnce(Option<(f64, f64, f32)>)>> =
            Box::from_raw(callback_ptr as *mut _);
        callback(Some((lat, lng, accuracy)));
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_drigster_taskupeatus_RustLocationHelper_onLocationError(
    _env: JNIEnv,
    _class: jni::objects::JClass,
    callback_ptr: i64,
) {
    unsafe {
        let callback: Box<Box<dyn FnOnce(Option<(f64, f64, f32)>)>> =
            Box::from_raw(callback_ptr as *mut _);
        callback(None);
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_drigster_taskupeatus_RustLocationHelper_onLocationChanged(
    _env: JNIEnv,
    _class: jni::objects::JClass,
    callback_ptr: i64,
    lat: f64,
    lng: f64,
    accuracy: f32,
) {
    unsafe {
        let callback = &*(callback_ptr as *const LocationUpdatesCallback);
        callback((lat, lng, accuracy));
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_drigster_taskupeatus_RustPermissionHelper_onPermissionResult(
    _env: JNIEnv,
    _class: jni::objects::JClass,
    callback_ptr: i64,
    granted: bool,
) {
    if let Ok(mut requests) = PERMISSION_REQUESTS.lock()
        && let Some(sender) = requests.remove(&callback_ptr)
    {
        let _ = sender.send(granted);
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_drigster_taskupeatus_RustLocationHelper_onLocationEnabledChanged(
    _env: JNIEnv,
    _class: jni::objects::JClass,
    callback_ptr: i64,
    enabled: bool,
) {
    unsafe {
        let callback = &*(callback_ptr as *const LocationEnabledCallback);
        callback(enabled);
    }
}
