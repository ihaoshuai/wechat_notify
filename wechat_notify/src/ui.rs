use std::time::{Duration, Instant};

use eframe::egui::{self, ViewportCommand};
use wechat_notify_common::{WECHAT_WINDOW_TITLE, get_atomicbool, set_atomicbool};
use windows_capture::{capture::GraphicsCaptureApiHandler, settings::{ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings, MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings}};

use crate::{capture::Capture, config, state, window::find_window};



pub struct App;

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // log::debug!("update");
        if ctx.has_requested_repaint() {
            // log::debug!("flash ui");

            let last_msg_time = state::LAST_MSG_TIME.load();
            if last_msg_time.is_none() || Instant::now() > last_msg_time.unwrap() + Duration::from_secs(config::SHOW_TIME) || !get_atomicbool(&state::IS_WECHAT_START) {
                ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
            }else {
                // log::debug!("show");
                ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
                if !get_atomicbool(&state::IS_CAPTURE_RUNNING) {
                    // 捕获微信窗口
                    let hwnd = find_window(WECHAT_WINDOW_TITLE).unwrap();
                    let window = windows_capture::window::Window::from_raw_hwnd(hwnd.0);
                    let capture_thread = std::thread::spawn(move || {
                        log::info!("start capture thread");
                        let settings = Settings::new(
                                    window,
                                    CursorCaptureSettings::Default,
                                    DrawBorderSettings::Default,
                                    SecondaryWindowSettings::Default,
                                    MinimumUpdateIntervalSettings::Default,
                                    DirtyRegionSettings::Default,
                                    ColorFormat::Rgba8,
                                    (),
                                );
                        Capture::start(settings).unwrap(); 
                    });
                    state::add_handle(capture_thread);
                    set_atomicbool(true, &state::IS_CAPTURE_RUNNING);
                }
                if let Some(frame) = state::FRAME.load().as_ref() {
                    let image = egui::ColorImage::from_rgba_unmultiplied([frame.width as usize, frame.height as usize], &frame.rgba);
                    let texture = ctx.load_texture("cap_frame", image, Default::default());
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.image(&texture);
                    });
                }
                ctx.request_repaint();
            }

        }
    }
}