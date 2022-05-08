// SPDX-License-Identifier: GPL-3.0-or-later

use crate::system::System;
use super::{Peripheral, nvic::irq};

#[derive(Default)]
pub struct Scb {
}

impl Scb {
    pub fn new(name: &str) -> Option<Box<dyn Peripheral>> {
        if name == "SCB" {
            Some(Box::new(Self::default()))
        } else {
            None
        }
    }
}

impl Peripheral for Scb {
    fn read(&mut self, _sys: &System, _offset: u32) -> u32 {
        0
    }

    fn write(&mut self, sys: &System, offset: u32, value: u32) {
        match offset {
            0x0004 => {
                // ICSR register
                // bit 26: set systick pending
                // bit 28: set PendSV pending
                if value & (1 << 26) != 0 {
                    sys.p.nvic.borrow_mut().set_intr_pending(irq::SYSTICK);
                }
                if value & (1 << 28) != 0 {
                    sys.p.nvic.borrow_mut().set_intr_pending(irq::PENDSV);
                }
            }
            _ => {}
        }
    }
}
