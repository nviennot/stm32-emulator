// SPDX-License-Identifier: GPL-3.0-or-later

use svd_parser::svd::{MaybeArray, RegisterInfo};
use unicorn_engine::Unicorn;

use super::Peripheral;

pub struct Rcc {
}

impl Rcc {
    pub fn use_peripheral(name: &str) -> bool {
        name == "RCC"
    }

    pub fn new(_name: String, _registers: &[MaybeArray<RegisterInfo>]) -> Self {
        Self {}
    }
}


impl Peripheral for Rcc {
    fn read(&mut self, _uc: &mut Unicorn<()>, offset: u32) -> u32 {
        if offset == 0 {
            // CR register
            // Return all the r to true. This is where the PLL ready flags are.
            //0b0010_0000_0010_0000_0000_0000_0010
            0xFFFF_FFFF
        } else if offset == 0x0008 {
            // CFGR register
            0b1000
        } else {
            0
        }
    }

    fn write(&mut self, _uc: &mut Unicorn<()>, _offset: u32, _value: u32) {
    }
}
