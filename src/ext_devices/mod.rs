// SPDX-License-Identifier: GPL-3.0-or-later

mod spi_flash;
mod usart_probe;

use spi_flash::{SpiFlashConfig, SpiFlash};
use usart_probe::{UsartProbeConfig, UsartProbe};

use std::convert::TryFrom;
use serde::Deserialize;
use anyhow::{Result, bail};

#[derive(Debug, Deserialize, Default)]
pub struct DevicesConfig {
   pub spi_flashes: Option<Vec<SpiFlashConfig>>,
   pub usart_probes: Option<Vec<UsartProbeConfig>>,
}

pub struct Devices {
    pub spi_flashes: Vec<SpiFlash>,
    pub usart_probes: Vec<UsartProbe>,
}

impl Devices {
    pub fn assert_empty(&self) -> Result<()> {
        for spi_flash in &self.spi_flashes {
            bail!("SPI Flash not used: {}", spi_flash.config.peripheral);
        }
        for usart_io in &self.usart_probes {
            bail!("USART IO not used: {}", usart_io.config.peripheral);
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

        let usart_probes = config.usart_probes.unwrap_or_default().into_iter()
            .map(|config| config.try_into())
            .collect::<Result<_>>()?;

        Ok(Self { spi_flashes, usart_probes })
    }
}
