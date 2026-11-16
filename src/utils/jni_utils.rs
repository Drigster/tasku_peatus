use blocking::unblock;
use std::collections::HashMap;

use jni::{
    Env, EnvUnowned, JavaVM, jni_sig, jni_str,
    objects::{JClass, JFloatArray, JObject, JString, JValue},
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{LazyLock, Mutex};

type LocationUpdatesCallback = Box<dyn Fn((f64, f64, f32)) + Send + Sync>;
type LocationEnabledCallback = Box<dyn Fn(bool) + Send + Sync>;
type LocationResultCallback = Box<dyn FnOnce(Option<(f64, f64, f32)>)>;

static PERMISSION_REQUEST_ID: AtomicI64 = AtomicI64::new(1);
static PERMISSION_REQUESTS: LazyLock<Mutex<HashMap<i64, std::sync::mpsc::Sender<bool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Attaches the current thread to the JVM and runs `f` with the Android
/// `Context`. A pending Java exception is cleared when `f` fails, so the next
/// JNI call on this thread doesn't abort.
fn with_android_env<T>(
    f: impl FnOnce(&mut Env, &JObject) -> jni::errors::Result<T>,
) -> Result<T, Box<dyn std::error::Error>> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) };

    vm.attach_current_thread(|env| {
        let context = unsafe { JObject::from_raw(env, ctx.context().cast()) };
        let result = f(env, &context);
        if result.is_err() && env.exception_check() {
            env.exception_describe();
            env.exception_clear();
        }
        result
    })
    .map_err(Into::into)
}

/// App classes aren't visible to `FindClass` from a native thread, so they are
/// loaded through the context's class loader.
fn load_app_class<'local>(
    env: &mut Env<'local>,
    context: &JObject,
    name: &str,
) -> jni::errors::Result<JClass<'local>> {
    let class_loader = env
        .call_method(
            context,
            jni_str!("getClassLoader"),
            jni_sig!("()Ljava/lang/ClassLoader;"),
            &[],
        )?
        .l()?;
    let class_name = env.new_string(name)?;
    let class = env
        .call_method(
            &class_loader,
            jni_str!("loadClass"),
            jni_sig!("(Ljava/lang/String;)Ljava/lang/Class;"),
            &[JValue::Object(&class_name)],
        )?
        .l()?;
    env.cast_local::<JClass>(class)
}

/// Calls a `static void method(Context, long)` on one of the app's helper classes.
fn call_helper(
    class_name: &str,
    method: &'static jni::strings::JNIStr,
    arg: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    with_android_env(|env, context| {
        let class = load_app_class(env, context, class_name)?;
        env.call_static_method(
            &class,
            method,
            jni_sig!("(Landroid/content/Context;J)V"),
            &[JValue::Object(context), JValue::Long(arg)],
        )?;
        Ok(())
    })
}

pub fn get_cache_dir_android() -> Result<PathBuf, Box<dyn std::error::Error>> {
    with_android_env(|env, context| {
        let file = env
            .call_method(
                context,
                jni_str!("getCacheDir"),
                jni_sig!("()Ljava/io/File;"),
                &[],
            )?
            .l()?;
        let path = env
            .call_method(
                &file,
                jni_str!("getAbsolutePath"),
                jni_sig!("()Ljava/lang/String;"),
                &[],
            )?
            .l()?;
        let path = env.cast_local::<JString>(path)?;
        let path: String = path.mutf8_chars(env)?.into();
        Ok(PathBuf::from(path))
    })
}

pub fn get_bar_sizes() -> Result<(f32, f32, f32, f32), Box<dyn std::error::Error>> {
    with_android_env(|env, context| {
        let class = load_app_class(env, context, "dev.drigster.taskupeatus.AndroidUiHelper")?;
        let insets = env
            .call_static_method(
                &class,
                jni_str!("getBarSizes"),
                jni_sig!("(Landroid/content/Context;)[F"),
                &[JValue::Object(context)],
            )?
            .l()?;

        if insets.is_null() {
            return Ok((0.0, 0.0, 0.0, 0.0));
        }

        let insets = env.cast_local::<JFloatArray>(insets)?;
        if insets.len(env)? < 4 {
            return Ok((0.0, 0.0, 0.0, 0.0));
        }

        let mut values = [0.0_f32; 4];
        insets.get_region(env, 0, &mut values)?;
        Ok((values[0], values[1], values[2], values[3]))
    })
}

pub async fn check_and_request_permissions() -> Result<bool, Box<dyn std::error::Error>> {
    let request_id = PERMISSION_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    let (tx, rx) = std::sync::mpsc::channel::<bool>();

    PERMISSION_REQUESTS
        .lock()
        .map_err(|_| "Permission requests lock poisoned")?
        .insert(request_id, tx);

    if let Err(err) = call_helper(
        "dev.drigster.taskupeatus.RustPermissionHelper",
        jni_str!("requestLocationPermission"),
        request_id,
    ) {
        if let Ok(mut requests) = PERMISSION_REQUESTS.lock() {
            requests.remove(&request_id);
        }
        return Err(err);
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
    let location = with_android_env(|env, context| {
        let location_service = env.new_string("location")?;
        let location_manager = env
            .call_method(
                context,
                jni_str!("getSystemService"),
                jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                &[JValue::Object(&location_service)],
            )?
            .l()?;

        for provider in ["gps", "network"] {
            let provider = env.new_string(provider)?;
            let location = env
                .call_method(
                    &location_manager,
                    jni_str!("getLastKnownLocation"),
                    jni_sig!("(Ljava/lang/String;)Landroid/location/Location;"),
                    &[JValue::Object(&provider)],
                )?
                .l()?;

            if !location.is_null() {
                let lat = env
                    .call_method(&location, jni_str!("getLatitude"), jni_sig!("()D"), &[])?
                    .d()?;
                let lng = env
                    .call_method(&location, jni_str!("getLongitude"), jni_sig!("()D"), &[])?
                    .d()?;
                return Ok(Some((lat, lng)));
            }
        }

        Ok(None)
    })?;

    location.ok_or_else(|| "No location available".into())
}

pub fn start_location_enabled_updates<F>(callback: F) -> Result<i64, Box<dyn std::error::Error>>
where
    F: Fn(bool) + Send + Sync + 'static,
{
    let callback_box: Box<LocationEnabledCallback> = Box::new(Box::new(callback));
    let callback_ptr = Box::into_raw(callback_box) as i64;

    if let Err(err) = call_helper(
        "dev.drigster.taskupeatus.RustLocationHelper",
        jni_str!("startLocationEnabledUpdates"),
        callback_ptr,
    ) {
        unsafe {
            drop(Box::from_raw(callback_ptr as *mut LocationEnabledCallback));
        }
        return Err(err);
    }

    Ok(callback_ptr)
}

// pub fn stop_location_enabled_updates(callback_ptr: i64) -> Result<(), Box<dyn std::error::Error>> {
//     call_helper(
//         "dev.drigster.taskupeatus.RustLocationHelper",
//         jni_str!("stopLocationEnabledUpdates"),
//         callback_ptr,
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
    let callback_box: Box<LocationUpdatesCallback> = Box::new(Box::new(callback));
    let callback_ptr = Box::into_raw(callback_box) as i64;

    if let Err(err) = call_helper(
        "dev.drigster.taskupeatus.RustLocationHelper",
        jni_str!("startLocationUpdates"),
        callback_ptr,
    ) {
        unsafe {
            drop(Box::from_raw(callback_ptr as *mut LocationUpdatesCallback));
        }
        return Err(err);
    }

    Ok(callback_ptr)
}

// pub fn stop_location_updates(callback_ptr: i64) -> Result<(), Box<dyn std::error::Error>> {
//     call_helper(
//         "dev.drigster.taskupeatus.RustLocationHelper",
//         jni_str!("stopLocationUpdates"),
//         callback_ptr,
//     )?;
//     unsafe {
//         drop(Box::from_raw(callback_ptr as *mut LocationUpdatesCallback));
//     }
//     Ok(())
// }

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_drigster_taskupeatus_RustLocationHelper_onLocationResult(
    _env: EnvUnowned,
    _class: JClass,
    callback_ptr: i64,
    lat: f64,
    lng: f64,
    accuracy: f32,
) {
    unsafe {
        let callback: Box<LocationResultCallback> = Box::from_raw(callback_ptr as *mut _);
        callback(Some((lat, lng, accuracy)));
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_drigster_taskupeatus_RustLocationHelper_onLocationError(
    _env: EnvUnowned,
    _class: JClass,
    callback_ptr: i64,
) {
    unsafe {
        let callback: Box<LocationResultCallback> = Box::from_raw(callback_ptr as *mut _);
        callback(None);
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_drigster_taskupeatus_RustLocationHelper_onLocationChanged(
    _env: EnvUnowned,
    _class: JClass,
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
    _env: EnvUnowned,
    _class: JClass,
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
    _env: EnvUnowned,
    _class: JClass,
    callback_ptr: i64,
    enabled: bool,
) {
    unsafe {
        let callback = &*(callback_ptr as *const LocationEnabledCallback);
        callback(enabled);
    }
}
