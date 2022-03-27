// SPDX-License-Identifier: GPL-3.0-or-later

use crate::system::System;
use super::Peripheral;

#[derive(Default)]
pub struct SysTick {
    reload: u32,
    val_toggle: bool,
}

impl SysTick {
    pub fn new(name: &str) -> Option<Box<dyn Peripheral>> {
        if name == "STK" {
            Some(Box::new(Self::default()))
        } else {
            None
        }
    }
}

impl Peripheral for SysTick {
    fn read(&mut self, _sys: &System, offset: u32) -> u32 {
        match offset {
            0x0004 => self.reload,
            0x0008 => {
                self.val_toggle = !self.val_toggle;
                if self.val_toggle { 0 } else { self.reload/2 }
            }
            _ => 0
        }
    }

    fn write(&mut self, _sys: &System, offset: u32, value: u32) {
        match offset {
            0x0004 => {
                // LOAD register
                self.reload = value
            }
            _ => {}
        }
    }
}
