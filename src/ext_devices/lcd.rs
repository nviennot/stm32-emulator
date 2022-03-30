// SPDX-License-Identifier: GPL-3.0-or-later

use std::{convert::TryFrom, collections::VecDeque};

use anyhow::Result;
use serde::Deserialize;

use crate::system::System;
use super::ExtDevice;

#[derive(Debug, Deserialize)]
pub struct LcdConfig {
    pub peripheral: String,
    pub width: u16,
    pub height: u16,
}

pub struct Lcd {
    pub config: LcdConfig,
    name: String,

    current_position: Point,
    framebuffer_raw: Vec<u8>,

    //reply: Option<Reply>,
    cmd: Option<(Command, Vec<u8>)>,
    drawing: bool,
}

#[derive(Default, Debug)]
pub struct Point {
    x: u16,
    y: u16,
}

impl Lcd {
    pub fn new(config: LcdConfig) -> Result<Self> {
        let mut framebuffer_raw = Vec::new();
        framebuffer_raw.resize(config.height as usize * config.width as usize, 0);

        Ok(Self {
            name: "".to_string(),
            config,
            current_position: Point::default(),
            framebuffer_raw,
            cmd: None,
            drawing: false,
        })
    }

    pub fn _write_framebuffer_to_file(&self, file: &str) -> Result<()> {
        use std::io::prelude::*;
        let mut f = std::fs::File::create(file)?;
        f.write_all(&self.framebuffer_raw)?;

        info!("Wrote framebuffer to {}", file);
        Ok(())
    }

    #[inline]
    fn get_framebuffer_raw_pixel(&mut self, x: u16, y: u16) -> &mut u8 {
        let x = x.min(self.config.width-1);
        let y = y.min(self.config.height-1);
        let x = x as usize;
        let y = y as usize;
        &mut self.framebuffer_raw[(x + y * self.config.width as usize)]
    }

    fn draw_pixel(&mut self, c: u8) {
        let Point { mut x, mut y } = self.current_position;
        let c = c << 4;
        *self.get_framebuffer_raw_pixel(x, y) = c;

        if x == 0 {
            debug!("{} p={},{} v={:02x}", self.name, x, y, c);
        }

        x += 1;
        if x >= self.config.width {
            x = 0;
            y += 1;

            if y >= self.config.height {
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
        /*
        match self.reply.as_mut() {
            Some(Reply::Data(d)) => {
                d.pop_front().unwrap_or_default()
            }

            Some(Reply::FileContent(addr)) => {
                let c = self.content[*addr];
                *addr = (*addr + 1) % self.config.size;
                c
            }
            None => 0,
        }
        */
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
