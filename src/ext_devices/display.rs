// SPDX-License-Identifier: GPL-3.0-or-later

use std::convert::TryFrom;

use anyhow::Result;
use serde::Deserialize;

use crate::peripherals::fsmc::{FsmcDevice, Bank};

#[derive(Debug, Deserialize)]
pub struct DisplayConfig {
    pub peripheral: String,
}

pub struct Display {
    pub config: DisplayConfig,
}

impl TryFrom<DisplayConfig> for Display {
    type Error = anyhow::Error;

    fn try_from(config: DisplayConfig) -> Result<Self> {
        Ok(Self { config })
    }
}

impl Display {
    fn cmd_pin(offset: u32) -> &'static str {
        if offset & (1 << (12+1)) != 0 {
            "data"
        } else {
            "cmd"
        }
    }
}

impl FsmcDevice for Display {
    fn name(&self, fsmc_bank_name: &str) -> String {
        format!("{} display", fsmc_bank_name)
    }

    fn read_data(&mut self, bank: &mut Bank, offset: u32) -> u32 {
        debug!("{} READ {}", bank.name, Self::cmd_pin(offset));
        0
    }

    fn write_data(&mut self, bank: &mut Bank, offset: u32, value: u32) {
        debug!("{} WRITE {} value=0x{:04x}", bank.name, Self::cmd_pin(offset), value as u16);
    }
}
