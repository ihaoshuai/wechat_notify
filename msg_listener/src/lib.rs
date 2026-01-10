use std::{ffi::c_void, path::Path, ptr::null_mut, sync::{OnceLock, atomic::{AtomicBool, AtomicPtr, Ordering}}, thread::sleep, time::Duration};

use anyhow::{Context, Result, anyhow, bail};
use wechat_notify_common::{NEW_MSG, PIPE_FILE, WECHAT_EXE_NAME, WECHAT_NEW_MSG_ID, WECHAT_NEW_MSG_WPARAM, get_atomicbool, set_atomicbool, to_pcstr, to_pcwstr};
use windows::{Win32::{Foundation::{CloseHandle, GENERIC_WRITE, HANDLE, HMODULE, LPARAM, LRESULT, TRUE, WPARAM}, Storage::FileSystem::{CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_NONE, OPEN_EXISTING, WriteFile}, System::{Diagnostics::Debug::OutputDebugStringA, LibraryLoader::{DisableThreadLibraryCalls, FreeLibraryAndExitThread, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GetModuleHandleExW}, Pipes::WaitNamedPipeW, SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH}, Threading::{CreateThread, GetCurrentProcessId, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_INFORMATION, QueryFullProcessImageNameW, THREAD_CREATE_RUN_IMMEDIATELY}}, UI::WindowsAndMessaging::{CWPRETSTRUCT, CallNextHookEx}}, core::{BOOL, PCWSTR, PWSTR}};

static RUNNING: AtomicBool = AtomicBool::new(false);
static HAS_NEW_MSG: AtomicBool = AtomicBool::new(false);
static PIPE: AtomicPtr<c_void> = AtomicPtr::new(null_mut());
static CURRENT_EXE: OnceLock<String> = OnceLock::new();


macro_rules! dll_try {
    ($expr:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => {
                let msg = format!("dll error: {}", e);
                OutputDebugStringA(to_pcstr(&msg));
                return BOOL(0);
            }
        }
    };
}



#[unsafe(no_mangle)]
pub extern "system" fn wechat_msg_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if ncode >= 0 {
            let msg = *(lparam.0 as *const CWPRETSTRUCT);
            if msg.message == WECHAT_NEW_MSG_ID && msg.wParam.0 == WECHAT_NEW_MSG_WPARAM {
                debug("wechat listener get new msg");
                HAS_NEW_MSG.store(true, Ordering::Relaxed);
            }
        }
        CallNextHookEx(None, ncode, wparam, lparam)
    }
}


#[unsafe(no_mangle)]
extern "system" fn DllMain(
    h_module: HMODULE,
    fdw_reason: u32,
    _lpv_reserved: *mut core::ffi::c_void
) -> BOOL {
    match fdw_reason {
        DLL_PROCESS_ATTACH => unsafe {
            dll_try!(CURRENT_EXE.set(dll_try!(get_current_exe_name())));
            debug(format!("DLL Attach in {}", CURRENT_EXE.get().unwrap()).as_str());
            if CURRENT_EXE.get().unwrap().eq(WECHAT_EXE_NAME) {
                dll_try!(DisableThreadLibraryCalls(h_module.into()));
                let mut h_self = HMODULE::default();
                dll_try!(GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, PCWSTR(h_module.0 as _), &mut h_self));
                set_atomicbool(true, &RUNNING);
                dll_try!(CreateThread(None, 0, Some(work_thread), Some(h_self.0), THREAD_CREATE_RUN_IMMEDIATELY, None));
            }
        },
        DLL_PROCESS_DETACH => unsafe {
            debug(format!("DLL Detach in {}", CURRENT_EXE.get().unwrap()).as_str());
            set_atomicbool(false, &RUNNING);
            let pipe = PIPE.swap(null_mut(), Ordering::Relaxed);
            if !pipe.is_null() {
                CloseHandle(HANDLE(pipe)).ok();
            }
        },
        _ => {}
    }
    TRUE
}

fn get_current_exe_name() -> Result<String> {
    unsafe {
        //获取完整路径
        let process_id = GetCurrentProcessId();
        let process = OpenProcess(PROCESS_QUERY_INFORMATION, false, process_id)?;
        let mut buffer = [0u16; 4096];
        let mut size = buffer.len() as u32;
        QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR::from_raw(buffer.as_mut_ptr()), &mut size)?;
        CloseHandle(process)?;
        let exe_path = String::from_utf16_lossy(&buffer[..size as usize]);

        //获取文件名
        let path = Path::new(&exe_path);
        if let Some(exe_name) = path.file_name() {
            let exe_name = exe_name.to_str().ok_or(anyhow!("fail to convert exe full path"))?;
            return Ok(exe_name.into());
        }


        bail!("get current exe name fail")
    }
}


unsafe extern "system" fn work_thread(lp_param: *mut c_void) -> u32{
    unsafe {
        let h_module = HMODULE(lp_param);

        debug("start connect pipe");
        while get_atomicbool(&RUNNING) {
            let pipe = PIPE.load(Ordering::Relaxed);
            if !pipe.is_null() {  
                if HAS_NEW_MSG.swap(false, Ordering::Relaxed) {
                    let pipe = HANDLE(pipe);
                    let mut written = 0;
                    if WriteFile(pipe, Some(&NEW_MSG), Some(&mut written), None).is_err() {
                        debug("fail to write pipe");
                        HAS_NEW_MSG.store(true, Ordering::Relaxed);
                        PIPE.store(null_mut(), Ordering::Relaxed);
                    }
                }
                sleep(Duration::from_millis(200));
                continue;
            }

            match connect_pipe() {
                Ok(pipe) => {
                    debug("connect to pipe");
                    PIPE.store(pipe.0, Ordering::Relaxed);
                },
                Err(e) => { 
                    debug(&format!("error : {}", e));
                    sleep(Duration::from_secs(1));
                }  
            }
        }
        debug("end connect pipe");
        FreeLibraryAndExitThread(h_module, 0);
    }
}

fn debug(s: &str) {
    unsafe { OutputDebugStringA(to_pcstr(s)) };
}


fn connect_pipe() -> Result<HANDLE>{
    unsafe {
        if !WaitNamedPipeW(to_pcwstr(PIPE_FILE), 3000).as_bool() {
            bail!("fail to wait pipe");
        }

        let pipe = CreateFileW(
            to_pcwstr(PIPE_FILE), 
            GENERIC_WRITE.0, 
            FILE_SHARE_NONE, 
            None, 
            OPEN_EXISTING, 
            FILE_ATTRIBUTE_NORMAL, 
            None
        ).context("fail to connect pipe")?;

        if pipe.is_invalid() {
            bail!("invalid pipe");
        }

        Ok(pipe)
    }
}

