// SPDX-License-Identifier: GPL-3.0-or-later

use std::{sync::Mutex, rc::Rc, cell::RefCell};

use sdl2::{
    event::Event,
    keyboard::Keycode,
    EventPump, VideoSubsystem, render::Canvas, video::Window, pixels,
};

lazy_static::lazy_static! {
    pub static ref SDL: Mutex<SdlEngine> = Mutex::new(SdlEngine::new());
}

pub struct SdlEngine {
    event_pump: EventPump,
    video_subsystem: VideoSubsystem,
}

/// How often should we call pump_events() in terms of number of instructions emulated
pub const PUMP_EVENT_INST_INTERVAL: u64 = 100_000; // ~1-10ms, depending on the speed of the host

unsafe impl Send for SdlEngine {}
unsafe impl Sync for SdlEngine {}

impl SdlEngine {
    pub fn new() -> Self {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();

        let event_pump = sdl_context.event_pump().unwrap();

        Self { event_pump, video_subsystem }
    }

    pub fn new_canvas(&mut self, title: &str, width: u32, height: u32) -> Canvas<Window> {
        let window = self.video_subsystem.window(title, width, height)
            .resizable()
            .build()
            .unwrap();

        let mut canvas = window.into_canvas().build().unwrap();

        canvas.set_draw_color(pixels::Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.present();

        canvas
    }

    /// Returns false if we need to quit
    pub fn pump_events(&mut self, framebuffers: &[Rc<RefCell<super::Sdl>>]) -> bool {
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Q), .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    return false;
                },
                Event::MouseMotion { ref window_id, .. } |
                Event::MouseButtonDown { ref window_id, .. } |
                Event::MouseButtonUp { ref window_id, .. } => {
                    if let Some(fb) = framebuffers.iter().find(|fb| fb.borrow().window_id == *window_id) {
                        fb.borrow_mut().process_event(event);
                    }
                }
                _ => {}
            }
        }
        true
    }
}
