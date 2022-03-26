// SPDX-License-Identifier: GPL-3.0-or-later

mod spi_flash;
use self::spi_flash::{SpiFlashConfig, SpiFlash};

use std::convert::TryFrom;
use serde::Deserialize;
use anyhow::{Result, bail};

#[derive(Debug, Deserialize, Default)]
pub struct DevicesConfig {
   pub spi_flashes: Option<Vec<SpiFlashConfig>>,
}

pub struct Devices {
    pub spi_flashes: Vec<SpiFlash>,
}

impl Devices {
    pub fn assert_empty(&self) -> Result<()> {
        for spi_flash in &self.spi_flashes {
            bail!("SPI Flash not used: {}", spi_flash.config.peripheral);
        }
        Ok(())
    }
}

impl TryFrom<DevicesConfig> for Devices {
    type Error = anyhow::Error;

    fn try_from(config: DevicesConfig) -> Result<Self> {
        let spi_flashes = config.spi_flashes.unwrap_or_default().into_iter()
            .map(|config| config.try_into())
            .collect::<Result<_>>()?;

        Ok(Self { spi_flashes })
    }
}
