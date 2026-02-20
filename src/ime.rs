use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use jni::objects::{GlobalRef, JClass, JObject};
use jni::JNIEnv;
use once_cell::sync::Lazy;
use parakeet_rs::{ExecutionConfig as OrtExecutionConfig, Nemotron};
use std::sync::{Arc, Mutex};

use crate::assets;
use crate::engine;

struct SendStream(#[allow(dead_code)] cpal::Stream);
unsafe impl Send for SendStream {}
unsafe impl Sync for SendStream {}

struct ImeState {
    stream: Option<SendStream>,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    jvm: Arc<jni::JavaVM>,
    service_ref: GlobalRef,
    is_streaming: Arc<std::sync::atomic::AtomicBool>,
}

static IME_STATE: Lazy<Mutex<Option<ImeState>>> = Lazy::new(|| Mutex::new(None));

#[no_mangle]
pub unsafe extern "system" fn Java_com_catfewd_nemotron_RustInputMethodService_initNative(
    env: JNIEnv,
    _class: JClass,
    service: JObject,
) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    let vm = env.get_java_vm().expect("Failed to get JavaVM");
    let vm_arc = Arc::new(vm);
    let service_ref = env.new_global_ref(&service).expect("Failed to ref service");

    let mut state_guard = IME_STATE.lock().unwrap();
    *state_guard = Some(ImeState {
        stream: None,
        audio_buffer: Arc::new(Mutex::new(Vec::new())),
        jvm: vm_arc.clone(),
        service_ref: service_ref.clone(),
        is_streaming: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    });

    let vm_clone = vm_arc.clone();
    let service_ref_clone = service_ref.clone();

    std::thread::spawn(move || {
        if engine::is_engine_loaded() {
            if let Ok(mut env) = vm_clone.attach_current_thread() {
                notify_status(&mut env, service_ref_clone.as_obj(), "Ready");
            }
            return;
        }

        if let Some(_guard) = engine::LoadingGuard::new() {
            if let Ok(mut env) = vm_clone.attach_current_thread() {
                let srv = service_ref_clone.as_obj();
                notify_status(&mut env, srv, "Loading model (Memory Mapped)...");

                match assets::get_mapped_assets(&mut env, srv) {
                    Ok(mapped) => {
                        let exec_cfg = OrtExecutionConfig::default();
                        
                        let application_info_obj = env.call_method(srv, "getApplicationInfo", "()Landroid/content/pm/ApplicationInfo;", &[]).unwrap().l().unwrap();
                        let source_dir_j = env.get_field(&application_info_obj, "sourceDir", "Ljava/lang/String;").unwrap().l().unwrap();
                        let apk_path: String = env.get_string(&source_dir_j.into()).unwrap().into();

                        let encoder_bytes = assets::get_asset_slice(&mapped.encoder, "assets/nemotron-model/encoder.onnx", &apk_path).unwrap();
                        let decoder_bytes = assets::get_asset_slice(&mapped.decoder, "assets/nemotron-model/decoder_joint.onnx", &apk_path).unwrap();

                        let chunk_size = {
                            let mut chunk_val = None;
                            let settings_str = env.new_string("settings").unwrap();
                            if let Ok(prefs) = env.call_method(
                                srv,
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
                                engine::set_engine(eng);
                                notify_status(&mut env, srv, "Ready");
                            }
                            Err(e) => {
                                notify_status(&mut env, srv, &format!("Error: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        notify_status(&mut env, srv, &format!("Error: {}", e));
                    }
                }
            }
        } else {
             if let Ok(mut env) = vm_clone.attach_current_thread() {
                let srv = service_ref_clone.as_obj();
                notify_status(&mut env, srv, "Waiting for model...");
                while engine::is_loading() {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
                if engine::is_engine_loaded() {
                    notify_status(&mut env, srv, "Ready");
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

#[no_mangle]
pub unsafe extern "system" fn Java_com_catfewd_nemotron_RustInputMethodService_cleanupNative(
    _env: JNIEnv,
    _class: JClass,
) {
    *IME_STATE.lock().unwrap() = None;
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_catfewd_nemotron_RustInputMethodService_startRecording(
    mut env: JNIEnv,
    _class: JClass,
) {
    let mut state_guard = IME_STATE.lock().unwrap();
    if let Some(state) = state_guard.as_mut() {
        let host = cpal::default_host();
        let device = match host.default_input_device() {
            Some(d) => d,
            None => {
                return;
            },
        };

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(16000),
            buffer_size: cpal::BufferSize::Default,
        };

        state.audio_buffer.lock().unwrap().clear();
        state.is_streaming.store(true, std::sync::atomic::Ordering::Release);

        let buffer_clone = state.audio_buffer.clone();
        let is_streaming_clone = state.is_streaming.clone();
        let jvm_clone = state.jvm.clone();
        let service_ref_clone = state.service_ref.clone();

        std::thread::spawn(move || {
            let mut env = jvm_clone.attach_current_thread().unwrap();
            let service_obj = service_ref_clone.as_obj();

            let engine_arc = match engine::get_engine() {
                Some(e) => e,
                None => {
                    return;
                },
            };

            let mut chunk_size = 8960;
            {
                if let Ok(eng) = engine_arc.lock() {
                    chunk_size = eng.get_chunk_size() * 160;
                }
            }

            {
                let mut eng = engine_arc.lock().unwrap();
                let _ = eng.reset();
            }

            while is_streaming_clone.load(std::sync::atomic::Ordering::Acquire) {
                let mut chunk_to_process = None;

                {
                    let mut shared_buffer = buffer_clone.lock().unwrap();
                    if shared_buffer.len() >= chunk_size {
                        chunk_to_process = Some(shared_buffer.drain(0..chunk_size).collect::<Vec<f32>>());
                    }
                }

                if let Some(chunk) = chunk_to_process {
                    let mut eng = engine_arc.lock().unwrap();
                    if let Ok(_delta_text) = eng.transcribe_chunk(&chunk) {
                        let full_text = eng.get_transcript();
                        if !full_text.is_empty() {
                            if let Ok(jtext) = env.new_string(&full_text) {
                                let _ = env.call_method(
                                    service_obj,
                                    "onPartialResult",
                                    "(Ljava/lang/String;)V",
                                    &[(&jtext).into()],
                                );
                            }
                        }
                    }
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }

            let engine_arc = match engine::get_engine() {
                Some(e) => e,
                None => return,
            };
            let final_text = {
                let eng = engine_arc.lock().unwrap();
                eng.get_transcript()
            };

            if !final_text.is_empty() {
                let formatted_text = crate::itn::format_text(&final_text);

                if let Ok(jtext) = env.new_string(&formatted_text) {
                    let _ = env.call_method(
                        service_obj,
                        "onTextTranscribed",
                        "(Ljava/lang/String;)V",
                        &[(&jtext).into()],
                    );
                }
            }
        });

        let buffer_for_capture = state.audio_buffer.clone();
        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _: &_| {
                buffer_for_capture.lock().unwrap().extend_from_slice(data);
            },
            |e| {
                // Silently ignore stream errors
                let _ = e;
            },
            None,
        );

        if let Ok(s) = stream {
            s.play().ok();
            state.stream = Some(SendStream(s));
            notify_status(&mut env, state.service_ref.as_obj(), "Listening...");
        }
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_catfewd_nemotron_RustInputMethodService_stopRecording(
    mut env: JNIEnv,
    _class: JClass,
) {
    let (_jvm, service_ref, _is_streaming) = {
        let mut state_guard = IME_STATE.lock().unwrap();
        if let Some(state) = state_guard.as_mut() {
            state.stream = None;
            state.is_streaming.store(false, std::sync::atomic::Ordering::Release);
            (
                state.jvm.clone(),
                state.service_ref.clone(),
                state.is_streaming.clone(),
            )
        } else {
            return;
        }
    };

    notify_status(&mut env, service_ref.as_obj(), "Ready");
}
