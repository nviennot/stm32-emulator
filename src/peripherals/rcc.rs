// SPDX-License-Identifier: GPL-3.0-or-later

use unicorn_engine::Unicorn;

use super::{Peripheral, Peripherals};

pub struct Rcc {
}

impl Rcc {
    pub fn new(name: &str) -> Option<Box<dyn Peripheral>> {
        if name == "RCC" {
            Some(Box::new(Rcc {}))
        } else {
            None
        }
    }
}


impl Peripheral for Rcc {
    fn read(&mut self, _perifs: &Peripherals, _uc: &mut Unicorn<()>, offset: u32) -> u32 {
        match offset {
            0x0000 => {
                // CR register
                // Return all the r to true. This is where the PLL ready flags are.
                //0b0010_0000_0010_0000_0000_0000_0010
                0xFFFF_FFFF
            }
            0x0008 => {
                // CFGR register
                0b1000
            }
            _ => 0
        }
    }

    fn write(&mut self, _perifs: &Peripherals, _uc: &mut Unicorn<()>, _offset: u32, _value: u32) {
    }
}
