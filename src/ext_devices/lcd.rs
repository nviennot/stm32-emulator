// SPDX-License-Identifier: GPL-3.0-or-later

use std::{convert::TryFrom, collections::VecDeque, rc::Rc, cell::RefCell};

use anyhow::Result;
use serde::Deserialize;

use crate::{system::System, framebuffers::{Framebuffers, Framebuffer, RGB888}};
use super::ExtDevice;

#[derive(Debug, Deserialize)]
pub struct LcdConfig {
    pub peripheral: String,
    pub framebuffer: String,
}

pub struct Lcd {
    pub config: LcdConfig,
    name: String,

    current_position: Point,
    width: u16,
    height: u16,
    // Can't figure out how to do grayscale with sdl2. See in sdl.rs.
    framebuffer: Rc<RefCell<dyn Framebuffer<RGB888>>>,

    cmd: Option<(Command, Vec<u8>)>,
    drawing: bool,
}

#[derive(Default, Debug)]
pub struct Point {
    x: u16,
    y: u16,
}

impl Lcd {
    pub fn new(config: LcdConfig, framebuffers: &Framebuffers) -> Result<Self> {
        let framebuffer = framebuffers.get(&config.framebuffer)?;
        let width = framebuffer.borrow().get_config().width;
        let height = framebuffer.borrow().get_config().height;

        Ok(Self {
            name: "".to_string(),
            config,
            current_position: Point::default(),
            framebuffer,
            width,
            height,
            cmd: None,
            drawing: false,
        })
    }

    #[inline]
    fn get_framebuffer_pixel_index(&mut self, x: u16, y: u16) -> usize {
        let x = x.min(self.width-1) as usize;
        let y = y.min(self.height-1) as usize;
        x + y * self.width as usize
    }

    fn draw_pixel(&mut self, c: u8) {
        let Point { mut x, mut y } = self.current_position;
        let i = self.get_framebuffer_pixel_index(x, y);
        let c = ((c as u32) << 4) | c as u32;
        let c = (c << 16) | (c << 8) | c;
        self.framebuffer.borrow_mut().get_pixels()[i] = c;

        /*
        if (x+y) % 100 == 0 {
            debug!("{} p={},{} v={:01x}", self.name, x, y, c);
        }
        */

        x += 1;
        if x >= self.width {
            x = 0;
            y += 1;

            if y >= self.height {
                y = 0;
            }
        }


        self.current_position = Point { x, y };
    }
}


impl ExtDevice<(), u8> for Lcd {
    fn connect_peripheral(&mut self, peri_name: &str) -> String {
        self.name = format!("{} LCD", peri_name);
        self.name.clone()
    }

    fn read(&mut self, _sys: &System, _addr: ()) -> u8 {
        0
    }

    fn write(&mut self, _sys: &System, _addr: (), v: u8) {
        if self.drawing {
            self.draw_pixel(v >> 4);
            self.draw_pixel(v & 0x0F);
            return;
        }

        if let Some((cmd, mut args)) = self.cmd.take() {
            // We are collecting a command argument
            args.push(v);
            if let Some(_reply) = self.try_process_command(cmd, &args) {
                //self.reply = Some(reply);
            } else {
                self.cmd = Some((cmd, args));
            }
        } else if let Some(cmd) = Command::try_from(v).ok() {
            // We are receiving a new command
            if let Some(_reply) = self.try_process_command(cmd, &[]) {
                //self.reply = Some(reply);
            } else {
                self.cmd = Some((cmd, vec![]));
            }
        } else if v != 0xff && v != 0x00 {
            warn!("{} unknown cmd={:02x}", self.name, v);
        }
    }
}

impl Lcd {
    /// Return some reply when the command is processed.
    /// None when command arguments are incomplete.
    fn try_process_command(&mut self, cmd: Command, args: &[u8]) -> Option<Reply> {
        match (cmd, args) {
            (Command::StartDrawing, []) => {
                self.current_position = Point::default();
                self.drawing = true;
                Some(Reply(vec![].into()))
            }
            _ => None,
        }.map(|reply| {
            debug!("{} cmd={:?} args={:02x?} reply={:02x?}",
                self.name, cmd, args, reply);
            reply
        })
    }
}


#[derive(Clone, Copy, Debug, num_enum::TryFromPrimitive)]
#[repr(u8)]
enum Command {
    GetVersion = 0xF0,
    StartDrawing = 0xFB,

    SetPalette = 0xF1,
    GetPalette = 0xF2,
}


#[derive(Debug)]
struct Reply(VecDeque<u8>);
