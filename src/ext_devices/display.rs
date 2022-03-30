// SPDX-License-Identifier: GPL-3.0-or-late

use std::{convert::TryFrom, rc::Rc, cell::RefCell};

use anyhow::Result;
use sdl2::{render::Canvas, video::Window, surface::Surface, pixels::PixelFormatEnum};
use serde::Deserialize;

use crate::{system::System, util::{Rect, Point}, sdl::Sdl};
use super::ExtDevice;

#[derive(Debug, Deserialize)]
pub struct DisplayConfig {
    pub peripheral: String,
    pub width: u16,
    pub height: u16,
    pub cmd_addr_bit: u32,
    pub swap_bytes: Option<bool>,
}

pub struct Display {
    pub config: DisplayConfig,
    name: String,
    draw_region: Rect,
    cmd: Option<(u8, Vec<u16>)>,
    drawing: bool,
    current_position: Point,
    framebuffer_raw: Vec<u16>,
    sdl: Rc<RefCell<Sdl>>,
    canvas: Canvas<Window>,
    framebuffer: Surface<'static>,
}

impl Display {
    pub fn new(config: DisplayConfig, sdl: &Rc<RefCell<Sdl>>) -> Result<Self> {
        let mut framebuffer_raw = Vec::new();
        framebuffer_raw.resize(config.height as usize * config.width as usize, 0);

        let canvas = sdl.borrow_mut().new_canvas(
            "display",
            config.width as u32,
            config.height as u32
        );

        let framebuffer = Surface::new(
            config.width.into(),
            config.height.into(),
            PixelFormatEnum::RGB565,
        ).unwrap();

        Ok(Self {
            name: "".to_string(),
            draw_region: Rect { left: 0, top: 0, right: config.width-1, bottom: config.height-1 },
            cmd: None,
            drawing: false,
            current_position: Point::default(),
            framebuffer_raw,
            config,
            sdl: sdl.clone(),
            canvas,
            framebuffer,
        })
    }

    pub fn write_framebuffer_to_file(&self, file: &str) -> Result<()> {
        use std::io::prelude::*;
        let mut b = vec![];
        for c in &self.framebuffer_raw {
            b.push(*c as u8);
            b.push((c >> 8) as u8);
        }

        let mut f = std::fs::File::create(file)?;
        f.write_all(&b)?;

        info!("Wrote framebuffer to {}", file);
        Ok(())
    }

    #[inline]
    fn get_framebuffer_pixel(&mut self, x: u16, y: u16) -> &mut u16 {
        let x = x as usize;
        let y = y as usize;

        let fb = self.framebuffer.without_lock_mut().unwrap();
        let fb_ptr = fb.as_mut_ptr() as *mut u16;

        unsafe {
            fb_ptr.add(x + y * self.config.width as usize).as_mut().unwrap()
        }
    }

    #[inline]
    fn get_framebuffer_raw_pixel(&mut self, x: u16, y: u16) -> &mut u16 {
        let x = x as usize;
        let y = y as usize;
        &mut self.framebuffer_raw[(x + y * self.config.width as usize)]
    }

    fn redraw(&mut self, sys: &System) {
        let tc = self.canvas.texture_creator();
        let texture = self.framebuffer.as_texture(&tc).unwrap();
        self.canvas.copy(&texture, None, None).unwrap();

        self.canvas.present();
        if !self.sdl.borrow_mut().pump_events() {
            sys.uc.borrow_mut().emu_stop().unwrap();
        }
    }

    fn draw_pixel(&mut self, c: u16) {
        let c = if self.config.swap_bytes.unwrap_or_default() {
            c.swap_bytes()
        } else {
            c
        };

        let Point { mut x, mut y } = self.current_position;
        *self.get_framebuffer_raw_pixel(x, y) = c;
        *self.get_framebuffer_pixel(x, y) = c;

        x += 1;
        if x > self.draw_region.right {
            x = self.draw_region.left;
            y += 1;

            if y > self.draw_region.bottom {
                y = self.draw_region.top;
            }
        }

        self.current_position = Point { x, y };

    }

    fn handle_cmd(&mut self) {
        if let Some((cmd, args)) = self.cmd.take() {
            match (Command::try_from(cmd).ok(), args.len()) {
                (Some(cmd @ Command::SetHoriRegion), 4) => {
                    let left  = (args[0] << 8) | args[1];
                    let right = (args[2] << 8) | args[3];
                    debug!("{} cmd={:?} left={} right={}", self.name, cmd, left, right);

                    self.draw_region.left = left.min(self.config.width-1);
                    self.draw_region.right = right.min(self.config.width-1);
                }
                (Some(cmd @ Command::SetVertRegion), 4) => {
                    let top    = (args[0] << 8) | args[1];
                    let bottom = (args[2] << 8) | args[3];
                    debug!("{} cmd={:?} top={} bottom={}", self.name, cmd, top, bottom);

                    self.draw_region.top = top.min(self.config.height-1);
                    self.draw_region.bottom = bottom.min(self.config.height-1);

                }
                (Some(Command::Draw), 0) => {
                    self.drawing = true;
                    self.current_position = Point {
                        x: self.draw_region.left,
                        y: self.draw_region.top,
                    }
                }
                _ => {
                    // Not the right time to consume, put it back
                    self.cmd = Some((cmd, args));
                }
            }
        }
    }

    fn finish_cmd(&mut self) {
        self.drawing = false;
        if let Some((cmd, args)) = self.cmd.take() {
            debug!("{} cmd=0x{:02x} args={:02x?}", self.name, cmd, args);
        }
    }
}

impl ExtDevice<u32, u32> for Display {
    fn connect_peripheral(&mut self, peri_name: &str) -> String {
        self.name = format!("{} display", peri_name);
        self.name.clone()
    }

    fn read(&mut self, _sys: &System, addr: u32) -> u32 {
        debug!("{} READ {:?}", self.name, Mode::from_addr(self.config.cmd_addr_bit, addr));
        self.finish_cmd();
        0
    }

    fn write(&mut self, sys: &System, addr: u32, value: u32) {
        let mode = Mode::from_addr(self.config.cmd_addr_bit, addr);
        trace!("{} WRITE {:?} value=0x{:04x}", self.name, mode, value as u16);
        match mode {
            Mode::Cmd => {
                self.finish_cmd();
                self.cmd = Some((value as u8, vec![]));
            }
            Mode::Data => {
                if self.drawing {
                    self.draw_pixel(value as u16);
                    if self.sdl.borrow_mut().should_redraw() || self.current_position == Point::default() {
                        self.redraw(sys);
                    }
                }
                 else if let Some((_cmd, args)) = self.cmd.as_mut() {
                    args.push(value as u16);
                }
            }
        }

        self.handle_cmd();
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Mode {
    Cmd,
    Data,
}


#[derive(Clone, Copy, Debug, num_enum::TryFromPrimitive)]
#[repr(u8)]
enum Command {
    SetHoriRegion = 0x2A,
    SetVertRegion = 0x2B,
    Draw = 0x2C,
}

impl Mode {
    fn from_addr(data_addr_bit: u32, offset: u32) -> Mode {
        if offset & data_addr_bit != 0 {
            Mode::Data
        } else {
            Mode::Cmd
        }
    }
}
