// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::VecDeque;
use std::convert::TryFrom;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{util, system::System};

use super::ExtDevice;

#[derive(Debug, Deserialize, Default)]
pub struct SpiFlashConfig {
    pub peripheral: String,
    pub jedec_id: u32,
    pub file: String,
    pub size: usize,
}

#[derive(Default)]
pub struct SpiFlash {
    pub config: SpiFlashConfig,
    name: String,
    content: Vec<u8>,

    reply: Option<Reply>,
    /// Command and arguments
    cmd: Option<(Command, Vec<u8>)>,
}

impl SpiFlash {
    pub fn new(config: SpiFlashConfig) -> Result<Self> {
        let mut content = util::read_file(&config.file)
            .with_context(|| format!("Failed to read {}", &config.file))?;

        content.resize(config.size, 0);

        Ok(Self { config, content, ..Self::default() })
    }
}

impl ExtDevice<(), u8> for SpiFlash {
    fn connect_peripheral(&mut self, peri_name: &str) -> String {
        self.name = format!("{} spi-flash", peri_name);
        self.name.clone()
    }

    fn read(&mut self, _sys: &System, _addr: ()) -> u8 {
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
    }

    fn write(&mut self, _sys: &System, _addr: (), v: u8) {
        if let Some((cmd, mut args)) = self.cmd.take() {
            // We are collecting a command argument
            args.push(v);
            if let Some(reply) = self.try_process_command(cmd, &args) {
                self.reply = Some(reply);
            } else {
                self.cmd = Some((cmd, args));
            }
        } else if let Some(cmd) = Command::try_from(v).ok() {
            // We are receiving a new command
            if let Some(reply) = self.try_process_command(cmd, &[]) {
                self.reply = Some(reply);
            } else {
                self.cmd = Some((cmd, vec![]));
            }
        } else if v != 0xff && v != 0x00 {
            debug!("{} unknown cmd={:02x}", self.name, v);
        }
    }
}

impl SpiFlash {
    /// Return some reply when the command is processed.
    /// None when command arguments are incomplete.
    fn try_process_command(&mut self, cmd: Command, args: &[u8]) -> Option<Reply> {
        match (cmd, args) {
            (Command::ReadJEDECID, []) => {
                let id = self.config.jedec_id;
                let data = id.to_be_bytes();
                Some(Reply::Data(data.into()))
            }
            (Command::ReadDeviceID, []) => {
                let id: u32 = 0xAABBCC;
                let data = id.to_be_bytes();
                Some(Reply::Data(data.into()))
            }
            (Command::ReadData, [a,b,c]) => {
                let mut addr = u32::from_be_bytes([0,*a,*b,*c]) as usize;

                if addr >= self.config.size {
                    warn!("{} cmd={:?} addr=0x{:06x} larger than size={:06x}",
                        self.name, cmd, addr, self.config.size);
                    addr = addr % self.config.size;
                }

                Some(Reply::FileContent(addr))
            }
            _ => None,
        }.map(|reply| {
            debug!("{} cmd={:?} args={:02x?} reply={:02x?}",
                self.name, cmd, args, reply);
            reply
        })
    }
}

#[derive(Debug, Clone, Copy, num_enum::TryFromPrimitive)]
#[repr(u8)]
enum Command {
    ReadData = 0x03,
    ReadJEDECID = 0x9F,
    ReadDeviceID = 0x90,
}

#[derive(Debug)]
enum Reply {
    FileContent(usize), // address
    Data(VecDeque<u8>),
}
