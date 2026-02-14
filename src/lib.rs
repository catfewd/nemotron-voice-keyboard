pub mod assets;
pub mod engine;
pub mod ime;
pub mod main_activity;
pub mod subtitle;

#[no_mangle]
pub unsafe extern "system" fn JNI_OnLoad(_vm: jni::JavaVM, _reserved: std::ffi::c_void) -> jni::sys::jint {
    std::panic::set_hook(Box::new(|info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        let location = info.location().map(|l| format!(" at {}:{}", l.file(), l.line())).unwrap_or_default();
        log::error!("RUST PANIC: {}{}", msg, location);
    }));
    jni::sys::JNI_VERSION_1_6
}
