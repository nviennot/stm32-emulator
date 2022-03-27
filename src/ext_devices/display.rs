// SPDX-License-Identifier: GPL-3.0-or-later

use std::convert::TryFrom;

use anyhow::Result;
use serde::Deserialize;

use crate::peripherals::fsmc::{FsmcDevice, Bank};

#[derive(Debug, Deserialize)]
pub struct DisplayConfig {
    pub peripheral: String,
    pub width: u16,
    pub height: u16,
}

pub struct Display {
    pub config: DisplayConfig,
    pub draw_region: Rect,
    pub cmd: Option<(u8, Vec<u16>)>,
    pub drawing: bool,
    pub current_position: Point,
    pub framebuffer_raw: Vec<u16>,
}

#[derive(Default, Debug)]
pub struct Point {
    x: u16,
    y: u16,
}

#[derive(Default, Debug)]
pub struct Rect {
    left: u16,
    right: u16,
    top: u16,
    bottom: u16,
}

impl TryFrom<DisplayConfig> for Display {
    type Error = anyhow::Error;

    fn try_from(config: DisplayConfig) -> Result<Self> {
        let mut framebuffer_raw = Vec::new();
        framebuffer_raw.resize(config.height as usize * config.width as usize, 0);

        let draw_region = Rect { left: 0, top: 0, right: config.width-1, bottom: config.height-1 };
        let current_position = Point::default();
        let cmd = None;
        let drawing = false;

        Ok(Self { config, draw_region, cmd, current_position, drawing, framebuffer_raw })
    }
}

impl Display {
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
    fn get_framebuffer_raw_pixel(&mut self, x: u16, y: u16) -> &mut u16 {
        let x = x.min(self.config.width-1);
        let y = y.min(self.config.height-1);
        let x = x as usize;
        let y = y as usize;
        &mut self.framebuffer_raw[(x + y * self.config.width as usize)]
    }

    fn draw_pixel(&mut self, c: u16) {
        let Point { mut x, mut y } = self.current_position;
        *self.get_framebuffer_raw_pixel(x, y) = c;

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

    fn handle_cmd(&mut self, bank: &Bank) {
        if let Some((cmd, args)) = self.cmd.take() {
            match (Command::try_from(cmd).ok(), args.len()) {
                (Some(cmd @ Command::SetHoriRegion), 4) => {
                    let left  = (args[0] << 8) | args[1];
                    let right = (args[2] << 8) | args[3];
                    self.draw_region.left = left;
                    self.draw_region.right = right;
                    debug!("{} cmd={:?} left={} right={}", bank.name, cmd, left, right);
                }
                (Some(cmd @ Command::SetVertRegion), 4) => {
                    let top    = (args[0] << 8) | args[1];
                    let bottom = (args[2] << 8) | args[3];
                    self.draw_region.top = top;
                    self.draw_region.bottom = bottom;
                    debug!("{} cmd={:?} top={} bottom={}", bank.name, cmd, top, bottom);
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

    fn finish_cmd(&mut self, bank: &Bank) {
        self.drawing = false;
        if let Some((cmd, args)) = self.cmd.take() {
            debug!("{} cmd=0x{:02x} args={:02x?}", bank.name, cmd, args);
        }
    }
}

impl FsmcDevice for Display {
    fn name(&self, fsmc_bank_name: &str) -> String {
        format!("{} display", fsmc_bank_name)
    }

    fn read_data(&mut self, bank: &mut Bank, offset: u32) -> u32 {
        debug!("{} READ {:?}", bank.name, Mode::from_addr(offset));
        self.finish_cmd(bank);
        0
    }

    fn write_data(&mut self, bank: &mut Bank, offset: u32, value: u32) {
        let mode = Mode::from_addr(offset);
        trace!("{} WRITE {:?} value=0x{:04x}", bank.name, mode, value as u16);
        match Mode::from_addr(offset) {
            Mode::Cmd => {
                self.finish_cmd(bank);
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

        self.handle_cmd(bank);
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
    fn from_addr(offset: u32) -> Mode {
        if offset & (1 << (12+1)) != 0 {
            Mode::Data
        } else {
            Mode::Cmd
        }
    }
}
