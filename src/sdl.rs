// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::{Instant, Duration};

use sdl2::{
    event::Event,
    keyboard::Keycode,
    EventPump, VideoSubsystem, render::Canvas, video::Window, pixels::Color,
};

pub struct Sdl {
    event_pump: EventPump,
    video_subsystem: VideoSubsystem,
    last_redraw: Instant,
}

impl Sdl {
    pub fn new() -> Self {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();

        let event_pump = sdl_context.event_pump().unwrap();
        let last_redraw = Instant::now();

        Self { event_pump, video_subsystem, last_redraw }
    }

    pub fn new_canvas(&mut self, title: &str, width: u32, height: u32) -> Canvas<Window> {
        let window = self.video_subsystem.window(title, width, height)
            .resizable()
            .build()
            .unwrap();

        let mut canvas = window.into_canvas().build().unwrap();

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.present();

        canvas
    }

    pub fn should_redraw(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last_redraw) > Duration::from_millis(10) {
            self.last_redraw = now;
            true
        } else {
            false
        }
    }

    /// Returns false if we need to quit
    pub fn pump_events(&mut self) -> bool {
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Q), .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    return false;
                },
                _ => {}
            }
        }
        true
    }
}
