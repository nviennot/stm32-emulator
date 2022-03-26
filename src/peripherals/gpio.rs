// SPDX-License-Identifier: GPL-3.0-or-later

use svd_parser::svd::{MaybeArray, RegisterInfo};
use unicorn_engine::Unicorn;

use super::Peripheral;

#[derive(Default)]
pub struct Gpio {
    moder: u32,
}

impl Gpio {
    pub fn use_peripheral(name: &str) -> bool {
        name.starts_with("GPIO")
    }

    pub fn new(_name: String, _registers: &[MaybeArray<RegisterInfo>]) -> Self {
        Self::default()
    }
}

impl Peripheral for Gpio {
    fn read(&mut self, _uc: &mut Unicorn<()>, offset: u32) -> u32 {
        match offset {
            0x0000 => self.moder,
            _ => 0,
        }
    }

    fn write(&mut self, _uc: &mut Unicorn<()>, _offset: u32, _value: u32) {
    }
}
