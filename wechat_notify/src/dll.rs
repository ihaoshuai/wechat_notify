use anyhow::{Context, bail, Result};
use wechat_notify_common::{to_pcstr, to_pcwstr};
use windows::Win32::{Foundation::{FreeLibrary, HMODULE, HWND}, System::LibraryLoader::{GetProcAddress, LoadLibraryW}, UI::WindowsAndMessaging::{GetWindowThreadProcessId, HHOOK, SetWindowsHookExW, UnhookWindowsHookEx, WH_GETMESSAGE}};

pub struct DllHandle {
    hhk: HHOOK,
    hmodule: HMODULE,
}

unsafe impl Send for DllHandle {}
unsafe impl Sync for DllHandle {}

impl DllHandle {
    pub fn new(hhk: HHOOK, hmodule: HMODULE) -> Self{
        DllHandle { hhk, hmodule }
    }
}

pub fn unload_dll(handle: DllHandle) -> Result<()>{
    log::info!("START: unload dll");
    unsafe { 
        UnhookWindowsHookEx(handle.hhk)?;
        FreeLibrary(handle.hmodule)?;
    };
    log::info!("SUCCEED: unload dll");
    Ok(())
}


pub fn inject_dll(dll: &str, proc_name: &str, target_hwmd: HWND) -> Result<DllHandle> {
    unsafe {
        let mut dll_path = std::env::current_exe()?;
        dll_path.pop();
        dll_path.push(dll);
        if !dll_path.exists() {
            bail!("inject dll fail: dll is not exist");
        }
        log::info!("START: inject dll -- {}", dll_path.to_str().unwrap());

        let h_inst = LoadLibraryW(to_pcwstr(format!("./{}", dll).as_str()))
                                .context("inject dll fail: can not load dll")?;
        let proc_addr = GetProcAddress(h_inst, to_pcstr(proc_name));
        if proc_addr.is_none() {
            bail!("inject dll fail: can not load hook function");
        }

        let dwthreadid = GetWindowThreadProcessId(target_hwmd, None);
        if dwthreadid == 0 {
            bail!("inject dll fail: can not find target window");
        }
        let h_hook = SetWindowsHookExW(WH_GETMESSAGE, std::mem::transmute(proc_addr.unwrap()), Some(h_inst.into()), dwthreadid)
                                                .context("inject dll fail: set hook fail")?;
        log::info!("SUCCEED: inject dll {}", dll);

        Ok(DllHandle::new(h_hook, h_inst))
    }
}