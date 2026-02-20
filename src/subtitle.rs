use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel;
use jni::objects::{JClass, JObject};
use jni::sys::{jfloat, jint};
use jni::JNIEnv;
use once_cell::sync::Lazy;

use crate::engine;
use parakeet_rs::Nemotron;

const SAMPLE_RATE: usize = 16_000;
const DEFAULT_CHUNK_SIZE: usize = 8_960; // ~560 ms at 16 kHz
const MIN_CHUNK_SIZE: usize = 2_560; // ~160 ms
const MAX_CHUNK_SIZE: usize = 16_000; // 1 second
const BUFFER_MULTIPLIER: usize = 12; // keep up to ~6.5 s of context
const DEFAULT_SILENCE_THRESHOLD: f32 = 0.0015;
const MAX_DISPLAY_CHARS: usize = 900;

struct LiveSubtitleState {
    buffer: Arc<Mutex<VecDeque<f32>>>,
    worker_tx: crossbeam_channel::Sender<Vec<f32>>,
    chunk_size: usize,
    silence_threshold: f32,
}

static LIVE_STATE: Lazy<Mutex<Option<LiveSubtitleState>>> = Lazy::new(|| Mutex::new(None));

#[no_mangle]
pub unsafe extern "system" fn Java_com_catfewd_nemotron_LiveSubtitleService_initNative(
    env: JNIEnv,
    _class: JClass,
    service: JObject,
) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    let vm = env.get_java_vm().expect("Failed to get JavaVM");
    let vm_arc = Arc::new(vm);
    let service_ref = env
        .new_global_ref(&service)
        .expect("Failed to create global ref for service");

    let (tx, rx) = crossbeam_channel::unbounded();

    {
        let mut guard = LIVE_STATE.lock().unwrap();
        *guard = Some(LiveSubtitleState {
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            worker_tx: tx.clone(),
            chunk_size: DEFAULT_CHUNK_SIZE,
            silence_threshold: DEFAULT_SILENCE_THRESHOLD,
        });
    }

    let vm_worker = vm_arc.clone();
    let service_ref_worker = service_ref.clone();

    thread::spawn(move || {
        let mut env = match vm_worker.attach_current_thread() {
            Ok(e) => e,
            Err(_) => {
                return;
            }
        };

        let service_obj = service_ref_worker.as_obj();

        let engine_arc = match wait_for_engine(Duration::from_secs(120)) {
            Some(arc) => arc,
            None => {
                return;
            }
        };

        {
            if let Ok(mut eng) = engine_arc.lock() {
                eng.reset();
            }
        }

        let mut aggregated_text = String::new();

        while let Ok(chunk) = rx.recv() {
            let new_text = {
                let mut eng = match engine_arc.lock() {
                    Ok(guard) => guard,
                    Err(_) => {
                        break;
                    }
                };

                match eng.transcribe_chunk(&chunk) {
                    Ok(txt) => txt,
                    Err(_) => {
                        continue;
                    }
                }
            };

            let trimmed = new_text.trim();
            if trimmed.is_empty() {
                continue;
            }

            if !aggregated_text.is_empty() {
                aggregated_text.push(' ');
            }
            aggregated_text.push_str(trimmed);
            trim_display_text(&mut aggregated_text);

            if let Ok(jtext) = env.new_string(&aggregated_text) {
                let _ = env.call_method(
                    service_obj,
                    "onSubtitleText",
                    "(Ljava/lang/String;)V",
                    &[(&jtext).into()],
                );
            }
        }

        if let Ok(mut eng) = engine_arc.lock() {
            let _ = eng.reset();
        };
    });
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_catfewd_nemotron_LiveSubtitleService_cleanupNative(
    _env: JNIEnv,
    _class: JClass,
) {
    *LIVE_STATE.lock().unwrap() = None;

    if let Some(engine_arc) = engine::get_engine() {
        if let Ok(mut eng) = engine_arc.lock() {
            eng.reset();
        }
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_catfewd_nemotron_LiveSubtitleService_setUpdateInterval(
    _env: JNIEnv,
    _class: JClass,
    interval_seconds: jfloat,
) {
    let mut guard = LIVE_STATE.lock().unwrap();
    if let Some(state) = guard.as_mut() {
        let seconds = if interval_seconds <= 0.0 {
            state.chunk_size as f32 / SAMPLE_RATE as f32
        } else {
            interval_seconds as f32
        };
        let desired_samples = (seconds * SAMPLE_RATE as f32) as usize;
        state.chunk_size = clamp_chunk_size(desired_samples);
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_catfewd_nemotron_LiveSubtitleService_pushAudio(
    env: JNIEnv,
    _class: JClass,
    data: jni::objects::JFloatArray,
    length: jint,
) {
    let len = length as usize;
    if len == 0 {
        return;
    }

    let mut incoming = vec![0f32; len];
    if let Err(_) = env.get_float_array_region(&data, 0, &mut incoming) {
        return;
    }

    let (buffer_arc, chunk_size, silence_threshold, tx) = {
        let guard = LIVE_STATE.lock().unwrap();
        if let Some(state) = guard.as_ref() {
            (
                state.buffer.clone(),
                state.chunk_size,
                state.silence_threshold,
                state.worker_tx.clone(),
            )
        } else {
            return;
        }
    };

    let mut queue = buffer_arc.lock().unwrap();
    queue.extend(incoming.iter().copied());

    let buffer_limit = chunk_size * BUFFER_MULTIPLIER;
    while queue.len() > buffer_limit {
        queue.pop_front();
    }

    let mut ready_chunks: Vec<Vec<f32>> = Vec::new();
    while queue.len() >= chunk_size {
        let mut chunk = Vec::with_capacity(chunk_size);
        for _ in 0..chunk_size {
            if let Some(sample) = queue.pop_front() {
                chunk.push(sample);
            }
        }

        if compute_rms(&chunk) >= silence_threshold {
            ready_chunks.push(chunk);
        }
    }
    drop(queue);

    for chunk in ready_chunks {
        if tx.send(chunk).is_err() {
            break;
        }
    }
}

fn wait_for_engine(timeout: Duration) -> Option<Arc<Mutex<Nemotron>>> {
    let start = Instant::now();
    loop {
        if let Some(engine_arc) = engine::get_engine() {
            return Some(engine_arc);
        }
        if start.elapsed() > timeout {
            return None;
        }
        thread::sleep(Duration::from_millis(200));
    }
}

fn clamp_chunk_size(samples: usize) -> usize {
    samples.clamp(MIN_CHUNK_SIZE, MAX_CHUNK_SIZE)
}

fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|v| v * v).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

fn trim_display_text(text: &mut String) {
    if text.len() > MAX_DISPLAY_CHARS {
        let drain = text.len() - MAX_DISPLAY_CHARS;
        text.drain(..drain);
    }
}
