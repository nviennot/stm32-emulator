// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::VecDeque;
use std::convert::TryFrom;

use anyhow::Result;
use serde::Deserialize;

use crate::peripherals::usart::{UsartDevice, UsartInner as Usart};

#[derive(Debug, Deserialize)]
pub struct UsartProbeConfig {
    pub peripheral: String,
}

pub struct UsartProbe {
    pub config: UsartProbeConfig,
}

impl TryFrom<UsartProbeConfig> for UsartProbe {
    type Error = anyhow::Error;

    fn try_from(config: UsartProbeConfig) -> Result<Self> {
        Ok(Self { config })
    }
}

impl UsartDevice for UsartProbe {
    fn name() -> Option<&'static str> {
        Some("usart-probe")
    }

    fn xfer(&mut self, usart: &mut Usart) -> Option<VecDeque<u8>> {
        if usart.tx.back() == Some(&0xa) {
            // end-of-line detected.
            let line = String::from_utf8_lossy(usart.tx.make_contiguous());
            let line = line.trim();
            info!("usart-probe p={} '{}'", usart.name, line);
            usart.tx.clear();
        }
        None
    }
}
