// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::{Instant, Duration};

use sdl2::mouse::MouseButton;
use sdl2::{pixels::PixelFormatEnum, surface::Surface, render::Canvas, video::Window};
use sdl2::{
    event::Event,
};

use super::{FramebufferConfig, Framebuffer, sdl_engine::SDL};

pub const REFRESH_DURATION_MILLIS: u64 = 20;

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
        let format = match config.mode.as_str() {
            "rgb565" => PixelFormatEnum::RGB565,
            // can't figure out how to do grayscale. See palette below.
            // "gray8" => PixelFormatEnum::Index8,
            "gray8" => PixelFormatEnum::RGB888,
            _ => unimplemented!(),
        };
        let mut canvas = SDL.lock().unwrap().new_canvas(
            &config.name,
            config.width.into(),
            config.height.into()
        );
        let framebuffer = Surface::new(
            config.width.into(),
            config.height.into(),
            format,
        ).unwrap();

        /*
        // Can't figure out how to use Index8.
        let colors: Vec<_> = (0..0xff).map(|u| Color::RGB(u, u, u)).collect();
        let palette = Palette::with_colors(&colors).unwrap();
        framebuffer.set_palette(&palette).unwrap();
        */

        if let Some(downscale) = config.downscale {
            canvas.window_mut().set_size(
                config.width as u32 / downscale,
                config.height as u32 / downscale,
            ).unwrap();
        }

        canvas.window_mut().raise();

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


impl<Color> Framebuffer<Color> for Sdl {
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
