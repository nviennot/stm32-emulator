// SPDX-License-Identifier: GPL-3.0-or-later

mod image;
use std::{rc::Rc, cell::RefCell};

use serde::Deserialize;

use self::image::Image;

#[derive(Debug, Deserialize)]
pub struct FramebufferConfig {
    pub name: String,
    pub width: u16,
    pub height: u16,
    pub mode: String,
    pub image_backend: Option<ImageBackendConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ImageBackendConfig {
    pub file: String,
}

pub type Color = u16; // RGB565 for now

pub trait Framebuffer {
    fn get_config(&self) -> &FramebufferConfig;

    /// Returns a mutable reference to the framebuffer
    fn get_pixels(&mut self) -> &mut [Color];

    /// Should be called once done modifying pixels
    fn invalidate(&mut self) {}

    /// for touch screens
    fn get_touch_position(&self) -> Option<(u16, u16)> { None }
}

pub struct Framebuffers {
    pub images: Vec<Rc<RefCell<Image>>>,
}

impl Framebuffers {
    pub fn from_config(mut config: Vec<FramebufferConfig>) -> Self {
        let mut images = vec![];

        for c in config.drain(..) {
            if c.image_backend.is_some() {
                images.push(Rc::new(RefCell::new(Image::new(c))));
            } else {
                panic!("no framebuffer backend specified");
            }
        }

        Self { images }
    }

    pub fn as_vec(&self) -> Vec<Rc<RefCell<dyn Framebuffer>>> {
        self.images.iter().map(|fb| fb.clone() as Rc<RefCell<dyn Framebuffer>>)
            .collect()
    }
}
