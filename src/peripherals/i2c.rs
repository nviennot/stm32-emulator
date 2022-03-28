// SPDX-License-Identifier: GPL-3.0-or-later

use crate::system::System;
use super::Peripheral;

#[derive(Default)]
pub struct I2c {
    toggle: u8,
}

impl I2c {
    pub fn new(name: &str) -> Option<Box<dyn Peripheral>> {
        if name.starts_with("I2C") {
            Some(Box::new(Self { ..I2c::default() }))
        } else {
            None
        }
    }
}

impl Peripheral for I2c {
    fn read(&mut self, _sys: &System, offset: u32) -> u32 {
        match offset {
            0x0014 => {
                // SR1
                self.toggle = (self.toggle + 1) % 4;
                if self.toggle & 2 != 0 { 0b111 } else { 0 }
            }
            0x0018 => {
                // SR2
                self.toggle = (self.toggle + 1) % 4;
                if self.toggle & 1  != 0{ 0b111 } else { 0 }
            }
            _ => 0
        }
    }

    fn write(&mut self, _sys: &System, _offset: u32, _value: u32) {
    }
}
