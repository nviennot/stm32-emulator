// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;
use std::rc::Rc;

use crate::ext_devices::{ExtDevices, ExtDevice};
use crate::system::System;
use super::Peripheral;

#[derive(Default)]
pub struct Usart {
    pub name: String,
    pub ext_device: Option<Rc<RefCell<dyn ExtDevice<(), u8>>>>,
}

impl Usart {
    pub fn new(name: &str, ext_devices: &ExtDevices) -> Option<Box<dyn Peripheral>> {
        if name.starts_with("USART") {
            let ext_device = ext_devices.find_serial_device(&name);
            let name = ext_device.as_ref()
                .map(|d| d.borrow_mut().connect_peripheral(name))
                .unwrap_or_else(|| name.to_string());
            Some(Box::new(Self { name, ext_device, ..Default::default() }))
        } else {
            None
        }
    }
}

impl Peripheral for Usart {
    fn read(&mut self, sys: &System, offset: u32) -> u32 {
        match offset {
            0x0000 => {
                // SR register
                // Bit 7 TXE: Transmit data register empty
                // Bit 6 TC: Transmission complete
                // Bit 5 RXNE: Read data register not empty
                // Bit 4 IDLE: IDLE line detected
                // We could do something smarter to indicate that there's data to read
                (1 << 7) | (1 << 6) | (1 << 5) | (1 << 4)
            }
            0x0004 => {
                // DR register
                let v = self.ext_device.as_ref().map(|d|
                    d.borrow_mut().read(sys, ())
                ).unwrap_or_default() as u32;

                trace!("{} read={:02x}", self.name, v);
                v
            }
            _ => 0
        }
    }

    fn write(&mut self, sys: &System, offset: u32, value: u32) {
        match offset {
            0x0004 => {
                // DR register
                self.ext_device.as_ref().map(|d|
                    d.borrow_mut().write(sys, (), value as u8)
                );

                trace!("{} write={:02x}", self.name, value as u8);
            }
            _ => {}
        }
    }
}
