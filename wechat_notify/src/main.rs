use std::{os::windows::process::CommandExt, process::Command, sync::{Arc, atomic::Ordering}, thread::sleep, time::{Duration, Instant}};

use anyhow::{Result, bail};
use eframe::egui;
use wechat_notify_common::{CREATE_NO_WINDOW, DLL_NAME, DLL_PROC_NAME, INJECT_TRY_TIME, PIPE_FILE, PUPPET_EXE_NAME, WECHAT_EXE_NAME, WECHAT_WINDOW_TITLE, get_atomicbool, set_atomicbool, to_pcwstr};
use windows::Win32::{Storage::FileSystem::{PIPE_ACCESS_INBOUND, ReadFile}, System::Pipes::{ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_MESSAGE, PIPE_TYPE_MESSAGE, PIPE_WAIT, PeekNamedPipe}, UI::WindowsAndMessaging::FindWindowW};

use crate::{ui::App, watch::{WatchControl, watch_start}};

mod dll;
mod ui;
mod config;
mod capture;
mod window;
mod state;
mod watch;

fn main() -> Result<()>{
    env_logger::init();
    //微信已启动
    window::run_if_window_exist(WECHAT_WINDOW_TITLE, |hwnd| {
        state::IS_WECHAT_START.store(true, Ordering::Relaxed);
        // let _ = dll::inject_dll(DLL_NAME, DLL_PROC_NAME, hwnd)?;
        let dll_handle = dll::inject_dll(DLL_NAME, DLL_PROC_NAME, hwnd)?;
        state::set_dll_handle(Some(dll_handle));
        Ok(())
    })?;

    //监听微信启动关闭
    let watch_thread = std::thread::spawn(|| {
        log::info!("watch thread start");
        let mut runing_state;
        while !get_atomicbool(&state::EXIT_FLAG) {
            if state::IS_WECHAT_START.load(Ordering::Relaxed) {
                log::info!("watching wechat close");
                runing_state = watch::watch_close(|process| {
                    if process.name == WECHAT_EXE_NAME {
                        log::info!("WeChat Close....");
                        state::IS_WECHAT_START.store(false, Ordering::Relaxed);
                        state::set_dll_handle(None);
                        return Ok(WatchControl::End);             
                    }
                    if process.name == PUPPET_EXE_NAME {
                        return Ok(WatchControl::End);
                    }
                    Ok(WatchControl::Continue)
                });
            }else {
                log::info!("watching wechat start");
                runing_state = watch_start(|process| {
                    if process.name == PUPPET_EXE_NAME {
                        return Ok(WatchControl::End);
                    }
                    if process.name == WECHAT_EXE_NAME {
                        log::info!("WeChat Start....");

                        for _ in 0..INJECT_TRY_TIME {
                            let hwnd = unsafe { FindWindowW(None, to_pcwstr(WECHAT_WINDOW_TITLE)) };
                            if hwnd.is_err() {
                                sleep(Duration::from_secs(1));
                                continue;
                            }                     
                            let hwnd = hwnd.unwrap();
                            // let _ = dll::inject_dll(DLL_NAME, DLL_PROC_NAME, hwnd)?;
                            let dll_handle = dll::inject_dll(DLL_NAME, DLL_PROC_NAME, hwnd)?;
                            state::set_dll_handle(Some(dll_handle));
                            state::IS_WECHAT_START.store(true, Ordering::Relaxed);
                            return Ok(WatchControl::End);
                        }
                        bail!("Fail to inject dll");
                    }
                    Ok(WatchControl::Continue)
                });
            }
            if let Err(e) = runing_state {
                log::error!("watch thread catch a error: {}", e);
                set_atomicbool(true, &state::EXIT_FLAG);
                break;
            }
        }
        log::info!("watch thread end")
    });

    state::add_handle(watch_thread);


    // let mut _input = String::new();
    // std::io::stdin().read_line(&mut _input)?;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
                        .with_always_on_top()
                        .with_decorations(false)
                        .with_position([config::SHOW_POSTION_X, config::SHOW_POSTION_Y])
                        .with_inner_size([config::MONITOR_W as f32, config::MONITOR_H as f32]),
        ..Default::default()
    };
    eframe::run_native("WeChat_Notify", options, Box::new(|cc| {
        let ctx = cc.egui_ctx.clone();
        //监听新消息
        let listen_msg_thread = std::thread::spawn(move || {
            unsafe {
                let pipe = CreateNamedPipeW(
                    to_pcwstr(PIPE_FILE), 
                    PIPE_ACCESS_INBOUND, 
                    PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT, 
                    1, 
                    1024, 
                    1024, 
                    0, 
                    None
                );

                while !get_atomicbool(&state::EXIT_FLAG) {
                    if  get_atomicbool(&state::IS_WECHAT_START){
                        ConnectNamedPipe(pipe, None).ok();
                        log::info!("dll already connect pipe");
                    }else {
                        // log::debug!("wait wechat connect pipe");
                        sleep(Duration::from_millis(500));
                        continue;
                    }

                    let mut buf = [0u8; 1];
                    let mut bytes_available = 0;
                    while !get_atomicbool(&state::EXIT_FLAG) {
                            match PeekNamedPipe(pipe, None, 0, None, Some(&mut bytes_available), None) {
                                Ok(_) => {
                                    if bytes_available == 0 {
                                        // log::debug!("wait new msg");
                                        sleep(Duration::from_millis(500));
                                        continue;
                                    }
                                },
                                Err(_e) => {
                                    log::info!("pipe disconnected");
                                    DisconnectNamedPipe(pipe).ok();
                                    sleep(Duration::from_secs(1));
                                    break;
                                }
                            }

                            match ReadFile(
                                pipe, 
                                Some(&mut buf), 
                                Some(&mut bytes_available),
                                None
                            ) {
                                Ok(_) => {
                                    if buf[0] == 1 {
                                        log::debug!("new msg");
                                        state::LAST_MSG_TIME.store(Arc::new(Some(Instant::now())));
                                        ctx.request_repaint();
                                    }
                                }, 
                                Err(_e) => {
                                    log::info!("read pipe error");
                                    break;
                                }
                            }
                    }

                }     
            }
        });
        state::add_handle(listen_msg_thread);


        Ok(Box::new(App))
    })).ok();



    set_atomicbool(true, &state::EXIT_FLAG);
    Command::new(PUPPET_EXE_NAME)
        .args(["/C", "timeout", "/T", "2", "/NOBREAK"])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;

    let mut dll_handle = state::DLL_HANDLE.lock().unwrap();
    if let Some(dll_handle) = dll_handle.take() {
        dll::unload_dll(dll_handle)?;
    }

    state::finish_threads();

    Ok(())
}


