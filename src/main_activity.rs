use jni::objects::{JClass, JObject};
use jni::JNIEnv;
use parakeet_rs::{ExecutionConfig as OrtExecutionConfig, Nemotron};
use std::sync::Arc;

use crate::assets;
use crate::engine;

#[no_mangle]
pub unsafe extern "system" fn Java_com_catfewd_nemotron_MainActivity_initNative(
    env: JNIEnv,
    _class: JClass,
    activity: JObject,
) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    crate::log_to_file("MainActivity: initNative called");

    let vm = env.get_java_vm().expect("Failed to get JavaVM");
    let vm_arc = Arc::new(vm);
    let activity_ref = env.new_global_ref(&activity).expect("Failed to ref activity");

    std::thread::spawn(move || {
        if let Ok(mut env) = vm_arc.attach_current_thread() {
            let act = activity_ref.as_obj();

            if engine::is_engine_loaded() {
                crate::log_to_file("MainActivity: Engine already loaded");
                notify_status(&mut env, &act, "Ready");
                return;
            }

            if let Some(_guard) = engine::LoadingGuard::new() {
                crate::log_to_file("MainActivity: Starting model load (direct memory map)");
                notify_status(&mut env, &act, "Loading model (Memory Mapped)...");
                log::info!("Starting model load from APK");

                match assets::get_mapped_assets(&mut env, &act) {
                    Ok(mapped) => {
                        let exec_cfg = OrtExecutionConfig::default();
                        
                        let application_info_obj = env.call_method(act, "getApplicationInfo", "()Landroid/content/pm/ApplicationInfo;", &[]).unwrap().l().unwrap();
                        let source_dir_j = env.get_field(&application_info_obj, "sourceDir", "Ljava/lang/String;").unwrap().l().unwrap();
                        let apk_path: String = env.get_string(&source_dir_j.into()).unwrap().into();

                        let encoder_bytes = match assets::get_asset_slice(&mapped.encoder, "assets/nemotron-model/encoder.onnx", &apk_path) {
                            Ok(b) => b,
                            Err(e) => {
                                let msg = format!("Failed to slice encoder: {}", e);
                                log::error!("{}", msg);
                                crate::log_to_file(&msg);
                                notify_status(&mut env, &act, &format!("Error: {}", e));
                                return;
                            }
                        };
                        let decoder_bytes = match assets::get_asset_slice(&mapped.decoder, "assets/nemotron-model/decoder_joint.onnx", &apk_path) {
                            Ok(b) => b,
                            Err(e) => {
                                let msg = format!("Failed to slice decoder: {}", e);
                                log::error!("{}", msg);
                                crate::log_to_file(&msg);
                                notify_status(&mut env, &act, &format!("Error: {}", e));
                                return;
                            }
                        };

                        let chunk_size = {
                            let mut chunk_val = None;
                            let settings_str = env.new_string("settings").unwrap();
                            if let Ok(prefs) = env.call_method(
                                act,
                                "getSharedPreferences",
                                "(Ljava/lang/String;I)Landroid/content/SharedPreferences;",
                                &[(&settings_str).into(), 0.into()],
                            ) {
                                let key_str = env.new_string("chunk_size").unwrap();
                                if let Ok(val) = env.call_method(
                                    prefs.l().unwrap(),
                                    "getInt",
                                    "(Ljava/lang/String;I)I",
                                    &[(&key_str).into(), 56.into()],
                                ) {
                                    chunk_val = Some(val.i().unwrap() as usize);
                                }
                            }
                            if let Some(v) = chunk_val {
                                crate::log_to_file(&format!("MainActivity: Using chunk size pref: {}", v));
                            }
                            chunk_val
                        };

                        match Nemotron::from_memory(
                            encoder_bytes,
                            decoder_bytes,
                            &mapped.tokenizer,
                            Some(exec_cfg),
                            chunk_size,
                        ) {
                            Ok(eng) => {
                                log::info!("Loaded model successfully from memory map");
                                crate::log_to_file("MainActivity: Model loaded successfully");
                                engine::set_engine(eng);
                                notify_status(&mut env, &act, "Ready");
                            }
                            Err(e) => {
                                let msg = format!("Model load failed: {}", e);
                                log::error!("{}", msg);
                                crate::log_to_file(&msg);
                                notify_status(&mut env, &act, &format!("Error: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        let msg = format!("Asset mapping failed: {}", e);
                        log::error!("{}", msg);
                        crate::log_to_file(&msg);
                        notify_status(&mut env, &act, &format!("Error: {}", e));
                    }
                }
            } else {
                crate::log_to_file("MainActivity: Waiting for existing load operation");
                notify_status(&mut env, &act, "Waiting for model...");
                while engine::is_loading() {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
                if engine::is_engine_loaded() {
                    crate::log_to_file("MainActivity: Engine became ready");
                    notify_status(&mut env, &act, "Ready");
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
