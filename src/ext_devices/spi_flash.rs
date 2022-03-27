// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::VecDeque;
use std::convert::TryFrom;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::util;
use crate::peripherals::spi::{SpiDevice, Spi};

#[derive(Debug, Deserialize)]
pub struct SpiFlashConfig {
    pub peripheral: String,
    pub jedec_id: u32,
    pub file: String,
    pub size: usize,
}

pub struct SpiFlash {
    pub config: SpiFlashConfig,
    pub content: Vec<u8>,
    pub read_addr: Option<usize>,
}

impl TryFrom<SpiFlashConfig> for SpiFlash {
    type Error = anyhow::Error;

    fn try_from(config: SpiFlashConfig) -> Result<Self> {
        let mut content = util::read_file(&config.file)
            .with_context(|| format!("Failed to read {}", &config.file))?;

        content.resize(config.size, 0);

        Ok(Self {
            config,
            content,
            read_addr: None,
        })
    }
}

impl SpiDevice for SpiFlash {
    fn name(&self, spi_name: &str) -> String {
        format!("{} ext-flash", spi_name)
    }

    fn xfer(&mut self, spi: &mut Spi) -> Option<VecDeque<u8>> {
        spi.tx.front().cloned().map(|cmd| {
            if cmd == 0xFF {
                spi.tx.pop_front();
                self.next_content_read(spi)
            } else if let Some(cmd) = Command::try_from(cmd).ok() {
                spi.rx = vec![].into();
                self.read_addr = None;
                self.handle_command(cmd, spi)
            } else {
                spi.rx = vec![].into();
                self.read_addr = None;
                debug!("{} tx={:02x?}", spi.name, spi.tx);
                None
            }
        }).flatten()
    }
}

impl SpiFlash {
    fn next_content_read(&mut self, spi: &Spi) -> Option<VecDeque<u8>> {
        if !spi.rx.is_empty() {
            return None;
        }

        self.read_addr.map(|read_addr| {
            const CHUNK_SIZE: usize = 16;
            let chunk = &self.content[read_addr..read_addr+CHUNK_SIZE];
            self.read_addr = Some(read_addr + CHUNK_SIZE);
            if chunk.is_empty() {
                None
            } else {
                Some(chunk.to_vec().into())
            }
        }).flatten()
    }

    fn handle_command(&mut self, cmd: Command, spi: &mut Spi) -> Option<VecDeque<u8>> {
        match cmd {
            Command::ReadJEDECID => {
                spi.tx.pop_front();
                info!("{} cmd={:?}", spi.name, cmd);
                let id = self.config.jedec_id;
                Some(vec![(id >> 16) as u8, (id >> 8) as u8, (id >> 0) as u8].into())
            }
            Command::ReadData if spi.tx.len() == 4 => {
                spi.tx.pop_front();

                let mut addr = 0;

                while let Some(a) = spi.tx.pop_front() {
                    addr = (addr << 8) | a as usize;
                }

                let addr = if addr > self.config.size {
                    warn!("{} cmd={:?} addr=0x{:06x} larger than size={:06x}",
                        spi.name, cmd, addr, self.config.size);
                    addr % self.config.size
                } else {
                    info!("{} cmd={:?} addr=0x{:06x}", spi.name, cmd, addr);
                    addr
                };

                self.read_addr = Some(addr);
                spi.rx = vec![].into();
                self.next_content_read(spi)
            }
            _ => None,
        }
    }
}

#[derive(Debug, num_enum::TryFromPrimitive)]
#[repr(u8)]
enum Command {
    ReadData = 0x03,
    ReadJEDECID = 0x9F,
}
