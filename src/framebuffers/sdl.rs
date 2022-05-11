// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::{Instant, Duration};

use sdl2::mouse::MouseButton;
use sdl2::{pixels::PixelFormatEnum, surface::Surface, render::Canvas, video::Window};
use sdl2::{
    event::Event,
};

use super::{FramebufferConfig, Framebuffer, Color, sdl_engine::SDL};

pub const REFRESH_DURATION_MILLIS: u64 = 20;

/*
lazy_static! {
    [pub] static ref NAME_1: TYPE_1 = EXPR_1;
    [pub] static ref NAME_2: TYPE_2 = EXPR_2;
    ...
    [pub] static ref NAME_N: TYPE_N = EXPR_N;
}
*/

pub struct Sdl {
    pub config: FramebufferConfig,
    canvas: Canvas<Window>,
    framebuffer: Surface<'static>,
    need_redraw: bool,
    last_redraw: Instant,
    pub window_id: u32,
    touch_position: Option<(u16, u16)>,
}

impl Sdl {
    pub fn new(config: FramebufferConfig) -> Self {
        let canvas = SDL.lock().unwrap().new_canvas(&config.name, config.width.into(), config.height.into());
        let framebuffer = Surface::new(
            config.width.into(),
            config.height.into(),
            PixelFormatEnum::RGB565,
        ).unwrap();

        let last_redraw = Instant::now();
        let need_redraw = false;
        let window_id = canvas.window().id();
        let touch_position = None;

        Self { config, canvas, framebuffer, need_redraw, last_redraw, window_id, touch_position }
    }

    fn should_redraw(&mut self) -> bool {
        if !self.need_redraw {
            return false;
        }

        let now = Instant::now();
        if now.duration_since(self.last_redraw) > Duration::from_millis(REFRESH_DURATION_MILLIS) {
            self.last_redraw = now;
            self.need_redraw = false;
            true
        } else {
            false
        }
    }

    pub fn maybe_redraw(&mut self) {
        if !self.should_redraw() {
            return;
        }

        let tc = self.canvas.texture_creator();
        let texture = self.framebuffer.as_texture(&tc).unwrap();
        self.canvas.copy(&texture, None, None).unwrap();

        self.canvas.present();
    }

    pub fn process_event(&mut self, event: Event) {
        match event {
            Event::MouseMotion { x, y, .. } => {
                if self.touch_position.is_some() {
                    self.touch_position = Some((x as u16, y as u16));
                }
            }
            Event::MouseButtonDown { mouse_btn: MouseButton::Left, x, y, .. } => {
                self.touch_position = Some((x as u16, y as u16));
            }
            Event::MouseButtonUp { mouse_btn:MouseButton::Left, .. } => {
                self.touch_position = None;
            }
            _ => {}
        }
    }
}


impl Framebuffer for Sdl {
    fn get_config(&self) -> &FramebufferConfig {
        &self.config
    }

    fn get_pixels(&mut self) -> &mut [Color] {
        self.need_redraw = true;

        let fb = self.framebuffer.without_lock_mut().unwrap();

        unsafe {
            std::slice::from_raw_parts_mut(
                fb.as_mut_ptr() as *mut Color,
                fb.len() / std::mem::size_of::<Color>(),
            )
        }
    }

    fn get_touch_position(&self) -> Option<(u16, u16)> {
        self.touch_position
    }
}
