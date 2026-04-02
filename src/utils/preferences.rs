use std::path::PathBuf;

// pub fn get_or<T>(name: &str, default: Value) -> Value
// where
//     T: std::str::FromStr + Clone,
// {
//     let root = Builder::new().prefix("simple-db").tempdir().unwrap();
//     fs::create_dir_all(root.path()).unwrap();
//     let path = root.path();

//     let mut manager = Manager::<SafeModeEnvironment>::singleton().write().unwrap();
//     let created_arc = manager.get_or_create(path, Rkv::new::<SafeMode>).unwrap();
//     let env = created_arc.read().unwrap();

//     // Then you can use the environment handle to get a handle to a datastore:
//     let store = env.open_single("mydb", StoreOptions::create()).unwrap();

//     // Use a write transaction to mutate the store via a `Writer`. There can be only
//     // one writer for a given environment, so opening a second one will block until
//     // the first completes.
//     let mut writer = env.write().unwrap();

//     // Keys are `AsRef<[u8]>`, while values are `Value` enum instances. Use the `Blob`
//     // variant to store arbitrary collections of bytes. Putting data returns a
//     // `Result<(), StoreError>`, where StoreError is an enum identifying the reason
//     // for a failure.
//     store.put(&mut writer, "int", &Value::I64(1234)).unwrap();
//     store
//         .put(&mut writer, "uint", &Value::U64(1234_u64))
//         .unwrap();
//     store
//         .put(&mut writer, "float", &Value::F64(1234.0.into()))
//         .unwrap();
//     store
//         .put(&mut writer, "instant", &Value::Instant(1528318073700))
//         .unwrap();
//     store
//         .put(&mut writer, "boolean", &Value::Bool(true))
//         .unwrap();
//     store
//         .put(&mut writer, "string", &Value::Str("Héllo, wörld!"))
//         .unwrap();
//     store
//         .put(
//             &mut writer,
//             "json",
//             &Value::Json(r#"{"foo":"bar", "number": 1}"#),
//         )
//         .unwrap();
//     store
//         .put(&mut writer, "blob", &Value::Blob(b"blob"))
//         .unwrap();

//     // You must commit a write transaction before the writer goes out of scope, or the
//     // transaction will abort and the data won't persist.
//     writer.commit().unwrap();
// }

pub fn get_cache_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    #[cfg(not(target_os = "android"))]
    {
        let path = dirs::cache_dir().unwrap();
        Ok(path)
    }
    #[cfg(target_os = "android")]
    {
        use crate::utils::jni_utils::get_cache_dir_android;
        get_cache_dir_android()
    }
}
