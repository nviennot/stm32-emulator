// SPDX-License-Identifier: GPL-3.0-or-later

mod spi_flash;
mod usart_probe;
mod display;

use spi_flash::{SpiFlashConfig, SpiFlash};
use usart_probe::{UsartProbeConfig, UsartProbe};
use display::{DisplayConfig, Display};

use std::{convert::TryFrom, rc::Rc, cell::RefCell};
use serde::Deserialize;
use anyhow::Result;

use crate::peripherals::{
    spi::SpiDevice,
    usart::UsartDevice,
    fsmc::FsmcDevice,
};

#[derive(Debug, Deserialize, Default)]
pub struct ExtDevicesConfig {
   pub spi_flashes: Option<Vec<SpiFlashConfig>>,
   pub usart_probes: Option<Vec<UsartProbeConfig>>,
   pub displays: Option<Vec<DisplayConfig>>,
}

pub struct ExtDevices {
    pub spi_flashes: Vec<Rc<RefCell<SpiFlash>>>,
    pub usart_probes: Vec<Rc<RefCell<UsartProbe>>>,
    pub displays: Vec<Rc<RefCell<Display>>>,
}

impl ExtDevices {
    pub fn find_spi_device(&self, peri_name: &str) -> Option<Rc<RefCell<dyn SpiDevice>>> {
        self.spi_flashes.iter()
            .filter(|d| d.borrow().config.peripheral == peri_name)
            .next()
            .map(|d| d.clone() as Rc<RefCell<dyn SpiDevice>>)
    }

    pub fn find_usart_device(&self, peri_name: &str) -> Option<Rc<RefCell<dyn UsartDevice>>> {
        self.usart_probes.iter()
            .filter(|d| d.borrow().config.peripheral == peri_name)
            .next()
            .map(|d| d.clone() as Rc<RefCell<dyn UsartDevice>>)
    }

    pub fn find_fsmc_device(&self, peri_name: &str) -> Option<Rc<RefCell<dyn FsmcDevice>>> {
        self.displays.iter()
            .filter(|d| d.borrow().config.peripheral == peri_name)
            .next()
            .map(|d| d.clone() as Rc<RefCell<dyn FsmcDevice>>)
    }
}

impl TryFrom<ExtDevicesConfig> for ExtDevices {
    type Error = anyhow::Error;

    fn try_from(config: ExtDevicesConfig) -> Result<Self> {
        let spi_flashes = config.spi_flashes.unwrap_or_default().into_iter()
            .map(|config| config.try_into().map(RefCell::new).map(Rc::new))
            .collect::<Result<_>>()?;

        let usart_probes = config.usart_probes.unwrap_or_default().into_iter()
            .map(|config| config.try_into().map(RefCell::new).map(Rc::new))
            .collect::<Result<_>>()?;

        let displays = config.displays.unwrap_or_default().into_iter()
            .map(|config| config.try_into().map(RefCell::new).map(Rc::new))
            .collect::<Result<_>>()?;

        Ok(Self { spi_flashes, usart_probes, displays })
    }
}
