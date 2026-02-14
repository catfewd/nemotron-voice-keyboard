use crate::{assets, engine};
use jni::objects::{JClass, JObject};
use jni::JNIEnv;
use parakeet_rs::{ExecutionConfig as OrtExecutionConfig, Nemotron};
use std::sync::Arc;

#[no_mangle]
pub unsafe extern "system" fn Java_dev_notune_transcribe_MainActivity_initNative(
    env: JNIEnv,
    _class: JClass,
    activity: JObject,
) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    // Initialize ORT if not already
    let _ = ort::init().commit();

    let vm = env.get_java_vm().expect("Failed to get JavaVM");
    let vm_arc = Arc::new(vm);
    let activity_ref = env
        .new_global_ref(&activity)
        .expect("Failed to ref activity");

    std::thread::spawn(move || {
        if let Ok(mut env) = vm_arc.attach_current_thread() {
            let act = activity_ref.as_obj();

            // 1. Check if already loaded
            if engine::is_engine_loaded() {
                notify_status(&mut env, act, "Ready");
                return;
            }

            // 2. Attempt to claim loading rights
            if let Some(_guard) = engine::LoadingGuard::new() {
                // We are the loader
                notify_status(&mut env, act, "Checking assets...");
                match assets::extract_assets(&mut env, act) {
                    Ok(path) => {
                        notify_status(&mut env, act, "Loading model (880MB)...");
                        log::info!("Starting model load from: {:?}", path);
                        
                        let exec_cfg = OrtExecutionConfig::new()
                            .with_intra_threads(2)
                            .with_inter_threads(1);
                        
                        match Nemotron::from_pretrained(&path, Some(exec_cfg)) {
                            Ok(engine_instance) => {
                                log::info!("Model loaded successfully");
                                engine::set_engine(engine_instance);
                                notify_status(&mut env, act, "Ready");
                            }
                            Err(e) => {
                                log::error!("Model load failed: {}", e);
                                notify_status(&mut env, act, &format!("Model Error: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Asset Error: {}", e);
                        notify_status(&mut env, act, &format!("Asset Error: {}", e));
                    }
                }
                // _guard drops here
            } else {
                // 3. Someone else is loading.
                notify_status(&mut env, act, "Waiting for model...");
                while engine::is_loading() {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
                if engine::is_engine_loaded() {
                    notify_status(&mut env, act, "Ready");
                } else {
                    notify_status(&mut env, act, "Model failed in other thread");
                }
            }
        }
    });
}

fn notify_status(env: &mut JNIEnv, obj: &JObject, msg: &str) {
    if let Ok(jmsg) = env.new_string(msg) {
        let _ = env.call_method(
            obj,
            "onStatusUpdate",
            "(Ljava/lang/String;)V",
            &[(&jmsg).into()],
        );
    }
}
