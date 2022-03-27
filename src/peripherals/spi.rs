// SPDX-License-Identifier: GPL-3.0-or-later


use crate::system::System;
use super::Peripheral;

use crate::ext_devices::ExtDevices;

use std::{collections::VecDeque, rc::Rc, cell::RefCell};

#[derive(Default)]
pub struct Spi {
    pub name: String,

    pub cr1: u32,
    pub bits16: bool,

    pub tx: VecDeque<u8>,
    pub rx: VecDeque<u8>,

    pub ext_device: Option<Rc<RefCell<dyn SpiDevice>>>,
}

impl Spi {
    pub fn new(name: &str, ext_devices: &ExtDevices) -> Option<Box<dyn Peripheral>> {
        if name.starts_with("SPI") {
            let ext_device = ext_devices.find_spi_device(name);
            let name = ext_device.as_ref()
                .map(|d| d.borrow().name(name))
                .unwrap_or_else(|| name.to_string());
            Some(Box::new(Self { name, ext_device, ..Default::default() }))
        } else {
            None
        }
    }
}

impl Peripheral for Spi {
    fn read(&mut self, _sys: &System, offset: u32) -> u32 {
        match offset {
            0x0000 => {
                self.cr1
            }
            0x0008 => {
                // SR register
                // receive buffer not empty
                // transmit buffer empty
                0b11
            }
            0x000C => {
                // DR register
                if self.bits16 {
                    if self.rx.len() >= 2 {
                        ((self.rx.pop_front().unwrap() as u32) << 8) |
                          self.rx.pop_front().unwrap() as u32
                    } else {
                        0
                    }
                } else {
                    self.rx.pop_front().unwrap_or(0) as u32
                }
            }
            _ => 0
        }
    }

    fn write(&mut self, _sys: &System, offset: u32, value: u32) {
        match offset {
            0x0000 => {
                // CR1 register
                self.bits16 = value & (1 << 11) != 0;
                self.cr1 = value;
            }
            0x000C => {
                // DR register
                if self.bits16 {
                    self.tx.push_back((value >> 8) as u8);
                }
                self.tx.push_back(value as u8);

                trace!("{} tx={:x?}", self.name, self.tx);

                if let Some(ext_device) = self.ext_device.as_mut() {
                    let rx = ext_device.clone().borrow_mut().xfer(self);
                    if let Some(rx) = rx {
                        debug!("{} rx={:x?}", self.name, &rx);
                        self.rx = rx;
                    }
                }
            }
            _ => {}
        }
    }
}

pub trait SpiDevice {
    fn name(&self, spi_name: &str) -> String;
    fn xfer(&mut self, spi: &mut Spi) -> Option<VecDeque<u8>>;
}
