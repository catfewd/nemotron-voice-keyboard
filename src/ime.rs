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
pub unsafe extern "system" fn Java_dev_notune_transcribe_RustInputMethodService_initNative(
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

    // Trigger lazy loading of engine if needed, but for IME we usually wait for main app.
    // However, if IME starts first (possible), we must ensure model is there.
    let vm_clone = vm_arc.clone();
    let service_ref_clone = service_ref.clone();

    std::thread::spawn(move || {
        // 1. Check if already loaded
        if engine::is_engine_loaded() {
            if let Ok(mut env) = vm_clone.attach_current_thread() {
                notify_status(&mut env, service_ref_clone.as_obj(), "Ready");
            }
            return;
        }

        // 2. Attempt to claim loading rights
        if let Some(_guard) = engine::LoadingGuard::new() {
            // We are the loader
            if let Ok(mut env) = vm_clone.attach_current_thread() {
                let srv = service_ref_clone.as_obj();
                
                notify_status(&mut env, srv, "Checking assets...");
                match assets::extract_assets(&mut env, srv) {
                    Ok(path) => {
                        notify_status(&mut env, srv, "Loading model (880MB)...");
                        log::info!("IME starting model load");

                        let exec_cfg = OrtExecutionConfig::new()
                            .with_intra_threads(2)
                            .with_inter_threads(1);
                        match Nemotron::from_pretrained(&path, Some(exec_cfg)) {
                            Ok(eng) => {
                                log::info!("IME loaded model successfully");
                                engine::set_engine(eng);
                                notify_status(&mut env, srv, "Ready");
                            }
                            Err(e) => {
                                log::error!("IME model load failed: {}", e);
                                notify_status(&mut env, srv, &format!("Error: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("IME asset extraction failed: {}", e);
                        notify_status(&mut env, srv, &format!("Error: {}", e));
                    }
                }
            }
            // _guard drops here
        } else {
             // 3. Someone else is loading.
             if let Ok(mut env) = vm_clone.attach_current_thread() {
                let srv = service_ref_clone.as_obj();
                notify_status(&mut env, srv, "Waiting for model...");
                
                while engine::is_loading() {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
                
                if engine::is_engine_loaded() {
                    notify_status(&mut env, srv, "Ready");
                } else {
                    notify_status(&mut env, srv, "Model failed in other thread");
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
pub unsafe extern "system" fn Java_dev_notune_transcribe_RustInputMethodService_cleanupNative(
    _env: JNIEnv,
    _class: JClass,
) {
    *IME_STATE.lock().unwrap() = None;
}

#[no_mangle]
pub unsafe extern "system" fn Java_dev_notune_transcribe_RustInputMethodService_startRecording(
    mut env: JNIEnv,
    _class: JClass,
) {
    let mut state_guard = IME_STATE.lock().unwrap();
    if let Some(state) = state_guard.as_mut() {
        let host = cpal::default_host();
        let device = match host.default_input_device() {
            Some(d) => d,
            None => return,
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

        // Spawn a worker thread to pump chunks to the engine
        std::thread::spawn(move || {
            let mut env = jvm_clone.attach_current_thread().unwrap();
            let service_obj = service_ref_clone.as_obj();
            
            let engine_arc = match engine::get_engine() {
                Some(e) => e,
                None => return,
            };

            {
                let mut eng = engine_arc.lock().unwrap();
                let _ = eng.reset();
            }

            const CHUNK_SIZE: usize = 8960; // 560ms

            while is_streaming_clone.load(std::sync::atomic::Ordering::Acquire) {
                let mut chunk_to_process = None;
                
                {
                    let mut shared_buffer = buffer_clone.lock().unwrap();
                    if shared_buffer.len() >= CHUNK_SIZE {
                        chunk_to_process = Some(shared_buffer.drain(0..CHUNK_SIZE).collect::<Vec<f32>>());
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

            // Send final result when streaming stops
            let engine_arc = match engine::get_engine() {
                Some(e) => e,
                None => return,
            };
            let final_text = {
                let eng = engine_arc.lock().unwrap();
                eng.get_transcript()
            };
            if !final_text.is_empty() {
                if let Ok(jtext) = env.new_string(&final_text) {
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
            |e| log::error!("Stream err: {}", e),
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
pub unsafe extern "system" fn Java_dev_notune_transcribe_RustInputMethodService_stopRecording(
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
