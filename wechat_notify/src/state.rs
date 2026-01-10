use std::{sync::{Arc, LazyLock, Mutex, atomic::AtomicBool}, thread::JoinHandle, time::Instant};

use arc_swap::ArcSwap;

use crate::{capture::SharedFrame, dll::DllHandle};

pub static IS_WECHAT_START: LazyLock<Arc<AtomicBool>> = LazyLock::new(|| Arc::new(AtomicBool::new(false)));
pub static EXIT_FLAG: AtomicBool = AtomicBool::new(false);

pub static DLL_HANDLE: Mutex<Option<DllHandle>> = Mutex::new(None);

pub static HANDLES: Mutex<Vec<JoinHandle<()>>> = Mutex::new(Vec::new());

pub static FRAME: LazyLock<ArcSwap<Option<SharedFrame>>> = LazyLock::new(|| ArcSwap::from_pointee(None));
pub static LAST_MSG_TIME: LazyLock<ArcSwap<Option<Instant>>> = LazyLock::new(|| ArcSwap::from_pointee(None));
pub static IS_CAPTURE_RUNNING: AtomicBool = AtomicBool::new(false);

pub fn set_dll_handle(dll_handle: Option<DllHandle>) {
    let mut dll_handle_store = DLL_HANDLE.lock().unwrap();
    *dll_handle_store = dll_handle;
}

pub fn add_handle(handle: JoinHandle<()>) {
    let mut handles = HANDLES.lock().unwrap();
    (*handles).push(handle);
}

pub fn pop_handle() {
    let mut handles = HANDLES.lock().unwrap();
    (*handles).pop();
}

pub fn finish_threads() {
    let mut handles = HANDLES.lock().unwrap();
    for handle in handles.drain(..) {
        handle.join().unwrap()
    }
}