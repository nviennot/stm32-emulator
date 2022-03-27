// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::ext_devices::ExtDevices;
use crate::system::System;
use super::Peripheral;

#[derive(Default)]
pub struct Usart {
    pub name: String,

    pub tx: VecDeque<u8>,
    pub rx: VecDeque<u8>,

    pub ext_device: Option<Rc<RefCell<dyn UsartDevice>>>,
}

impl Usart {
    pub fn new(name: &str, ext_devices: &ExtDevices) -> Option<Box<dyn Peripheral>> {
        if name.starts_with("USART") {
            let ext_device = ext_devices.find_usart_device(&name);
            let name = ext_device.as_ref()
                .map(|d| d.borrow().name(name))
                .unwrap_or_else(|| name.to_string());
            Some(Box::new(Self { name, ext_device, ..Default::default() }))
        } else {
            None
        }
    }
}

impl Peripheral for Usart {
    fn read(&mut self, _sys: &System, offset: u32) -> u32 {
        match offset {
            0x0004 => {
                // DR register
                self.rx.pop_front().unwrap_or(0) as u32
            }
            _ => 0
        }
    }

    fn write(&mut self, _sys: &System, offset: u32, value: u32) {
        match offset {
            0x0004 => {
                // DR register
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

pub trait UsartDevice {
    fn name(&self, usart_name: &str) -> String;
    fn xfer(&mut self, usart: &mut Usart) -> Option<VecDeque<u8>>;
}
