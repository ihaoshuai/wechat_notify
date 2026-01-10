
use anyhow::{Ok, Result, bail};
use wechat_notify_common::to_pcwstr;
use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::FindWindowW};


pub fn run_if_window_exist<F, T>(window_title: &str, run: F) -> Result<Option<T>>
where 
    F: FnOnce(HWND) -> Result<T>
{
    let hwnd_res = unsafe { 
        FindWindowW(None, to_pcwstr(window_title))
    }; 

    if hwnd_res.is_err() {
        return Ok(None);
    }

    let hwnd = hwnd_res.unwrap();
    if hwnd.is_invalid() {
        return Ok(None);
    }

    let res = run(hwnd)?;

    Ok(Some(res))
}


pub fn find_window(window_title: &str) -> Result<HWND> {
    let hwnd = unsafe { 
        FindWindowW(None, to_pcwstr(window_title))?
    };

    if hwnd.is_invalid() {
        bail!("find a invalid window : {}", window_title);
    }

    Ok(hwnd)
}