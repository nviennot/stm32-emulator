// SPDX-License-Identifier: GPL-3.0-or-later

use std::{convert::TryFrom, collections::VecDeque, rc::Rc, cell::RefCell};

use anyhow::Result;
use serde::Deserialize;

use crate::{system::System, util::{Rect, Point}, framebuffers::{Framebuffer, Framebuffers, RGB565}};
use super::ExtDevice;

#[derive(Debug, Deserialize)]
pub struct DisplayConfig {
    pub peripheral: String,
    pub cmd_addr_bit: u32,
    pub swap_bytes: Option<bool>,
    pub replies: Option<Vec<ReplyConfig>>,
    pub framebuffer: String,
}

#[derive(Debug, Deserialize)]
pub struct ReplyConfig {
    pub cmd: u8,
    data: Vec<u16>,
}

pub struct Display {
    pub config: DisplayConfig,
    name: String,
    draw_region: Rect,
    cmd: Option<(u8, Vec<u16>)>,
    reply: VecDeque<u16>,
    drawing: bool,
    current_position: Point,
    width: u16,
    height: u16,
    framebuffer: Rc<RefCell<dyn Framebuffer<RGB565>>>,
}

impl Display {
    pub fn new(config: DisplayConfig, framebuffers: &Framebuffers) -> Result<Self> {
        let framebuffer = framebuffers.get(&config.framebuffer)?;
        let width = framebuffer.borrow().get_config().width;
        let height = framebuffer.borrow().get_config().height;

        Ok(Self {
            name: "?".to_string(), // This is filled out on connect_peripheral()
            draw_region: Rect { left: 0, top: 0, right: width-1, bottom: height-1 },
            cmd: None,
            reply: Default::default(),
            drawing: false,
            current_position: Point::default(),
            width, height,
            framebuffer: framebuffer.clone(),
            config,
        })
    }

    #[inline]
    fn get_framebuffer_pixel_index(&mut self, x: u16, y: u16) -> usize {
        let x = x.min(self.width-1) as usize;
        let y = y.min(self.height-1) as usize;
        x + y * self.width as usize
    }


    fn draw_pixel(&mut self, c: u16) {
        let c = if self.config.swap_bytes.unwrap_or_default() {
            c.swap_bytes()
        } else {
            c
        };

        let Point { mut x, mut y } = self.current_position;
        let i = self.get_framebuffer_pixel_index(x, y);
        self.framebuffer.borrow_mut().get_pixels()[i] = c;

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
                    self.draw_region.left = left;
                    self.draw_region.right = right;
                    debug!("{} cmd={:?} left={} right={}", self.name, cmd, left, right);
                }
                (Some(cmd @ Command::SetVertRegion), 4) => {
                    let top    = (args[0] << 8) | args[1];
                    let bottom = (args[2] << 8) | args[3];
                    self.draw_region.top = top;
                    self.draw_region.bottom = bottom;
                    debug!("{} cmd={:?} top={} bottom={}", self.name, cmd, top, bottom);
                }
                (Some(cmd @ Command::Draw), 0) => {
                    self.drawing = true;
                    self.current_position = Point {
                        x: self.draw_region.left,
                        y: self.draw_region.top,
                    };
                    debug!("{} cmd={:?}", self.name, cmd);
                }
                _ => {
                    // If we need to reply to a read, put it there.
                    if let Some(replies) = self.config.replies.as_ref() {
                        if let Some(reply) = replies.iter().find(|r| r.cmd == cmd) {
                            self.reply = reply.data.iter().cloned().collect();
                            debug!("{} cmd={:02x?} reply={:02x?}", self.name, cmd, reply.data);
                            return;
                        }
                    }

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
        let mode = Mode::from_addr(self.config.cmd_addr_bit, addr);

        let v = match mode {
            Mode::Cmd => {
                0
            }
            Mode::Data => {
                self.reply.pop_front().unwrap_or_default()
            }
        };

        trace!("{} READ {:?} -> {:02x}", self.name, mode, v);
        v as u32
    }

    fn write(&mut self, _sys: &System, addr: u32, value: u32) {
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
                } else if let Some((_cmd, args)) = self.cmd.as_mut() {
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
