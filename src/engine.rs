use once_cell::sync::Lazy;
use parakeet_rs::Nemotron;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub static GLOBAL_ENGINE: Lazy<Mutex<Option<Arc<Mutex<Nemotron>>>>> =
    Lazy::new(|| Mutex::new(None));

pub static IS_LOADING: AtomicBool = AtomicBool::new(false);

pub fn get_engine() -> Option<Arc<Mutex<Nemotron>>> {
    GLOBAL_ENGINE.lock().unwrap().clone()
}

pub fn set_engine(engine: Nemotron) {
    *GLOBAL_ENGINE.lock().unwrap() = Some(Arc::new(Mutex::new(engine)));
}

pub fn is_engine_loaded() -> bool {
    GLOBAL_ENGINE.lock().unwrap().is_some()
}

pub fn try_claim_loading() -> bool {
    IS_LOADING
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_ok()
}

pub fn finish_loading() {
    IS_LOADING.store(false, Ordering::Release);
}

pub fn is_loading() -> bool {
    IS_LOADING.load(Ordering::Acquire)
}

pub struct LoadingGuard;

impl LoadingGuard {
    pub fn new() -> Option<Self> {
        if try_claim_loading() {
            Some(Self)
        } else {
            None
        }
    }
}

impl Drop for LoadingGuard {
    fn drop(&mut self) {
        finish_loading();
    }
}
