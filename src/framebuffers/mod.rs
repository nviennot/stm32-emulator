// SPDX-License-Identifier: GPL-3.0-or-later

pub mod image;
pub mod sdl;
pub mod sdl_engine;

use std::{rc::Rc, cell::RefCell};
use serde::Deserialize;
use self::{image::Image, sdl::Sdl};
use anyhow::Result;

#[derive(Debug, Deserialize)]
pub struct FramebufferConfig {
    pub name: String,
    pub width: u16,
    pub height: u16,
    pub mode: String,
    pub image: Option<ImageBackendConfig>,
    pub sdl: Option<bool>,
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

    /// for touch screens
    fn get_touch_position(&self) -> Option<(u16, u16)> { None }
}

pub struct Framebuffers {
    pub images: Vec<Rc<RefCell<Image>>>,
    pub sdls: Vec<Rc<RefCell<Sdl>>>,
}

impl Framebuffers {
    pub fn from_config(mut config: Vec<FramebufferConfig>) -> Self {
        let mut images = vec![];
        let mut sdls = vec![];

        for c in config.drain(..) {
            match (c.image.is_some(), c.sdl == Some(true)) {
                (true, false) => images.push(Rc::new(RefCell::new(Image::new(c)))),
                (false, true) => sdls.push(Rc::new(RefCell::new(Sdl::new(c)))),
                (false, false) => panic!("no framebuffer backend specified. Use image or sdl"),
                _ => panic!("Multiple backend specified"),
            }
        }

        Self { images, sdls }
    }

    pub fn get(&self, name: &str) -> Result<Rc<RefCell<dyn Framebuffer>>> {
        let images = self.images.iter().map(|fb| fb.clone() as Rc<RefCell<dyn Framebuffer>>);
        let sdls = self.sdls.iter().map(|fb| fb.clone() as Rc<RefCell<dyn Framebuffer>>);
        let fb = images.chain(sdls).find(|fb| fb.borrow().get_config().name == name);
        fb.ok_or(anyhow::anyhow!("Cannot find framebuffer {}", name))
    }
}
