// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::VecDeque;
use unicorn_engine::Unicorn;
use crate::ext_devices::Devices;

use super::{Peripheral, Peripherals};

pub struct Usart<D: UsartDevice> {
    inner: UsartInner,
    device: D,
}

#[derive(Default)]
pub struct UsartInner {
    pub name: String,

    pub tx: VecDeque<u8>,
    pub rx: VecDeque<u8>,
}

impl Usart<GenericUsartDevice> {
    pub fn new(name: &str, devices: &mut Devices) -> Option<Box<dyn Peripheral>> {
        if let Some(p) = Self::new_generic(name) {
            let device_index = devices.usart_probes.iter().enumerate()
                .filter(|(_,f)| f.config.peripheral == name)
                .map(|(i,_)| i)
                .next();

            if let Some(device_index) = device_index {
                let d = devices.usart_probes.swap_remove(device_index);
                Some(Box::new(p.with_device(d)))
            } else {
                Some(Box::new(p))
            }

        } else {
            None
        }

    }

    pub fn new_generic(name: &str) -> Option<Self> {
        if name.starts_with("USART") {
            let name = name.to_string();
            Some(Self {
                inner: UsartInner { name, ..Default::default() },
                device: GenericUsartDevice,
            })
        } else {
            None
        }
    }

    pub fn with_device<D: UsartDevice>(mut self, device: D) -> Usart<D> {
        if let Some(device_name) = D::name() {
            self.inner.name.push_str(" ");
            self.inner.name.push_str(device_name);
        }
        Usart { inner: self.inner, device }
    }
}

impl<D: UsartDevice> Peripheral for Usart<D> {
    fn read(&mut self, _perifs: &Peripherals, _uc: &mut Unicorn<()>, offset: u32) -> u32 {
        match offset {
            0x0004 => {
                // DR register
                self.inner.rx.pop_front().unwrap_or(0) as u32
            }
            _ => 0
        }
    }

    fn write(&mut self, _perifs: &Peripherals, _uc: &mut Unicorn<()>, offset: u32, value: u32) {
        match offset {
            0x0004 => {
                // DR register
                self.inner.tx.push_back(value as u8);

                let rx = self.device.xfer(&mut self.inner);
                if let Some(rx) = rx {
                    debug!("{} rx={:x?}", self.inner.name, &rx);
                    self.inner.rx = rx;
                }
            }
            _ => {}
        }
    }
}

pub trait UsartDevice: Sized {
    fn name() -> Option<&'static str> { None }
    fn xfer(&mut self, usart: &mut UsartInner) -> Option<VecDeque<u8>>;
}

#[derive(Default)]
pub struct GenericUsartDevice;

impl UsartDevice for GenericUsartDevice {
    fn xfer(&mut self, usart: &mut UsartInner) -> Option<VecDeque<u8>> {
        debug!("{} tx={:x?}", usart.name, usart.tx);
        None
    }
}
