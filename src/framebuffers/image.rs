// SPDX-License-Identifier: GPL-3.0-or-later

use std::{io::BufWriter, fs::File};
use super::{FramebufferConfig, Framebuffer, Color};
use anyhow::Result;


pub struct Image {
    pub config: FramebufferConfig,
    pub framebuffer: Vec<Color>,
}

impl Image {
    pub fn new(config: FramebufferConfig) -> Self {
        let mut framebuffer = vec![];
        framebuffer.resize(config.width as usize * config.height as usize, Default::default());
        Self { config, framebuffer }
    }

    pub fn get_framebuffer_as_rgb(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity((self.config.width * self.config.height * 3).into());

        for c in self.framebuffer.iter().cloned() {
            // RGB565
            let r = (c >> 11) * 0xFF / 0b11111;
            let g = ((c >> 5) & 0b111111) * 0xFF / 0b111111;
            let b = (c & 0b11111) * 0xFF / 0b11111;

            v.push(r as u8);
            v.push(g as u8);
            v.push(b as u8);
        }

        v
    }

    pub fn write_to_disk(&self) -> Result<()> {
        let path = &self.config.image_backend.as_ref().unwrap().file;
        let file = File::create(path).unwrap();
        let ref mut w = BufWriter::new(file);

        let mut encoder = png::Encoder::new(w, self.config.width.into(), self.config.height.into());
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header().unwrap();

        writer.write_image_data(&self.get_framebuffer_as_rgb()).unwrap();

        info!("Wrote framebuffer to {}", path);

        Ok(())
    }
}

// Note: Drop doesn't work because I think Unicorn doesn't cleanup closures correctly.

impl Framebuffer for Image {
    fn get_config(&self) -> &FramebufferConfig {
        &self.config
    }

    fn get_pixels(&mut self) -> &mut [Color] {
        &mut self.framebuffer
    }
}
