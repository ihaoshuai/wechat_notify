
use std::sync::{Mutex, atomic::{AtomicBool, Ordering}};

use windows::{Win32::UI::WindowsAndMessaging::WM_USER, core::{PCSTR, PCWSTR}};

pub const CUSTOM_MSG_ID: u32 = WM_USER + 100;
pub const WECHAT_NEW_MSG_ID: u32 = 0x0118;
pub const WECHAT_NEW_MSG_WPARAM: usize = 0xFFF8;
pub const NOTIFY_WINDOW_TITLE: &str = "WeChat Notify";
pub const WECHAT_WINDOW_TITLE: &str = "微信";
pub const DLL_NAME: &str = "msg_listener.dll";
pub const DLL_PROC_NAME: &str = "wechat_msg_proc";
pub const APP_EXE_NAME: &str = "wechat_notify.exe";
pub const WECHAT_EXE_NAME: &str = "Weixin.exe";
pub const PUPPET_EXE_NAME: &str = "cmd.exe";
pub const PIPE_FILE: &str = r"\\.\pipe\wechat_notify";
pub const NEW_MSG: [u8; 1] = [1];
pub const INJECT_TRY_TIME: u8 = 10;

pub const CREATE_NO_WINDOW: u32 = 0x08000000;


pub fn to_pcwstr(original: &str) -> PCWSTR {
    let vec: Vec<u16> = original.encode_utf16().chain(std::iter::once(0)).collect();
    PCWSTR::from_raw(vec.as_ptr())
}

pub fn to_pcstr(original: &str) -> PCSTR{
    PCSTR::from_raw(format!("{}\0", original).as_ptr())
}

pub fn set_atomicbool(val: bool, atomicbool: &AtomicBool) {
    atomicbool.store(val, Ordering::Relaxed);
}
pub fn get_atomicbool(atomicbool: &AtomicBool) -> bool{
    atomicbool.load(Ordering::Relaxed)
}

pub fn store<T>(target: Mutex<T>, val: T) {
    let mut guard = target.lock().unwrap();
    *guard = val;
}

pub fn load<T: Clone>(target: Mutex<T>) -> T {
    let guard = target.lock().unwrap();
    (*guard).clone()
}


