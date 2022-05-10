// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::{Instant, Duration};

use sdl2::{pixels::PixelFormatEnum, surface::Surface, render::Canvas, video::Window};

use super::{FramebufferConfig, Framebuffer, Color, sdl_engine::SDL};

pub const REFRESH_DURATION_MILLIS: u64 = 10;

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

        Self { config, canvas, framebuffer, need_redraw, last_redraw }
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
        None
    }
}
