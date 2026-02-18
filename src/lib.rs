pub mod assets;
pub mod engine;
pub mod ime;
pub mod itn;
pub mod main_activity;
pub mod subtitle;

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use once_cell::sync::Lazy;

static LOG_FILE: Lazy<Mutex<Option<std::fs::File>>> = Lazy::new(|| Mutex::new(None));

pub fn log_to_file(msg: &str) {
    let mut file_guard = LOG_FILE.lock().unwrap();
    
    // Initialize if needed
    if file_guard.is_none() {
        // Try to open external storage path first (accessible via /sdcard)
        let path = "/sdcard/Android/data/com.catfewd.nemotron/files/nemotron_debug.log";
        // Create directory if it doesn't exist
        let _ = std::fs::create_dir_all("/sdcard/Android/data/com.catfewd.nemotron/files");
        
        match OpenOptions::new().create(true).append(true).open(path) {
            Ok(f) => *file_guard = Some(f),
            Err(e) => {
                // Fallback to internal app storage if permission denied
                let internal_path = "/data/data/com.catfewd.nemotron/files/nemotron_debug.log";
                if let Ok(f) = OpenOptions::new().create(true).append(true).open(internal_path) {
                    *file_guard = Some(f);
                }
            }
        }
    }

    if let Some(file) = file_guard.as_mut() {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(file, "[{}] {}", timestamp, msg);
    }
}

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
        let full_msg = format!("RUST PANIC: {}{}", msg, location);
        log::error!("{}", full_msg);
        log_to_file(&full_msg);
    }));
    jni::sys::JNI_VERSION_1_6
}
