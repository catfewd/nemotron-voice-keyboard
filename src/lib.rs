pub mod assets;
pub mod engine;
pub mod ime;
pub mod itn;
pub mod main_activity;
pub mod subtitle;

#[no_mangle]
pub unsafe extern "system" fn JNI_OnLoad(_vm: jni::JavaVM, _reserved: std::ffi::c_void) -> jni::sys::jint {
    std::panic::set_hook(Box::new(|info| {
        // Silent panic - no logging for privacy
        let _ = info;
    }));
    jni::sys::JNI_VERSION_1_6
}
