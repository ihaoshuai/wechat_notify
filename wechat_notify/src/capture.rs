use std::{sync::Arc, time::{Duration, Instant}};

use wechat_notify_common::{get_atomicbool, set_atomicbool};
use windows_capture::capture::GraphicsCaptureApiHandler;

use crate::{config, state};

pub struct SharedFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub struct Capture;

impl GraphicsCaptureApiHandler for Capture {
    type Flags = ();
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn on_frame_arrived(
            &mut self,
            frame: &mut windows_capture::frame::Frame,
            capture_control: windows_capture::graphics_capture_api::InternalCaptureControl,
        ) -> Result<(), Self::Error> {
        
        let last_msg_time = state::LAST_MSG_TIME.load().as_ref().unwrap();
        if !get_atomicbool(&state::IS_WECHAT_START) || get_atomicbool(&state::EXIT_FLAG) || Instant::now() > last_msg_time + Duration::from_secs(config::SHOW_TIME){
            log::info!("capture thread exit");
            set_atomicbool(false, &state::IS_CAPTURE_RUNNING);
            capture_control.stop();
            state::pop_handle();
            return Ok(());
        }

        let mut buffer = frame.buffer_crop(
            config::MONITOR_X, config::MONITOR_Y, 
            config::MONITOR_X + config::MONITOR_W, config::MONITOR_Y + config::MONITOR_H)?;
        let rgba = buffer.as_nopadding_buffer()?.to_vec();
        state::FRAME.store(Arc::new(Some(SharedFrame { width: config::MONITOR_W, height: config::MONITOR_H, rgba })));

        Ok(())
    }

    fn new(_ctx: windows_capture::capture::Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self)
    }
}