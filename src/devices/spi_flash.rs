// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::VecDeque;
use std::convert::TryFrom;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::util;
use crate::peripherals::spi::{SpiDevice, SpiInner as Spi};

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
}

impl TryFrom<SpiFlashConfig> for SpiFlash {
    type Error = anyhow::Error;

    fn try_from(config: SpiFlashConfig) -> Result<Self> {
        let content = util::read_file(&config.file)
            .with_context(|| format!("Failed to read {}", &config.file))?;

        Ok(Self {
            config,
            content,
        })
    }
}

impl SpiDevice for SpiFlash {
    fn name() -> Option<&'static str> {
        Some("Flash")
    }

    fn xfer(&mut self, spi: &mut Spi) -> Option<VecDeque<u8>> {
        let cmd = spi.tx.pop_front().unwrap_or(0xFF);
        if let Ok(cmd) = Command::try_from(cmd) {
            debug!("{} cmd={:?}", spi.name, cmd);
            Some(self.handle_command(cmd, spi))
        } else if cmd != 0xFF {
            debug!("{} unknown_cmd={:02x}", spi.name, cmd);
            None
        } else {
            None
        }
    }
}

impl SpiFlash {
    fn handle_command(&mut self, cmd: Command, _spi: &mut Spi) -> VecDeque<u8> {
        match cmd {
            Command::ReadJEDECID => {
                let id = self.config.jedec_id;
                vec![(id >> 16) as u8, (id >> 8) as u8, (id >> 0) as u8].into()
            }
            Command::ReadData => {
                vec![].into()
            }
        }
    }
}

#[derive(Debug, num_enum::TryFromPrimitive)]
#[repr(u8)]
enum Command {
    ReadData = 0x03,
    ReadJEDECID = 0x9F,
}
