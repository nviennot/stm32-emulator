// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::VecDeque;

use unicorn_engine::Unicorn;

use crate::devices::Devices;

use super::Peripheral;

pub struct Spi<D: SpiDevice> {
    inner: SpiInner,
    device: D,
}

#[derive(Default)]
pub struct SpiInner {
    pub name: String,

    pub cr1: u32,
    pub bits16: bool,

    pub tx: VecDeque<u8>,
    pub rx: VecDeque<u8>,
}

impl Spi<GenericSpiDevice> {
    pub fn new(name: &str, devices: &mut Devices) -> Option<Box<dyn Peripheral>> {
        if let Some(p) = Self::new_generic(name) {
            let device_index = devices.spi_flashes.iter().enumerate()
                .filter(|(_,f)| f.config.peripheral == name)
                .map(|(i,_)| i)
                .next();

            if let Some(device_index) = device_index {
                let d = devices.spi_flashes.swap_remove(device_index);
                Some(Box::new(p.with_device(d)))
            } else {
                Some(Box::new(p))
            }

        } else {
            None
        }

    }

    pub fn new_generic(name: &str) -> Option<Self> {
        if name.starts_with("SPI") {
            let name = name.to_string();
            Some(Self {
                inner: SpiInner { name, ..Default::default() },
                device: GenericSpiDevice,
            })
        } else {
            None
        }
    }

    pub fn with_device<D: SpiDevice>(mut self, device: D) -> Spi<D> {
        if let Some(device_name) = D::name() {
            self.inner.name.push_str(" ");
            self.inner.name.push_str(device_name);
        }
        Spi { inner: self.inner, device }
    }
}

impl<D: SpiDevice> Peripheral for Spi<D> {
    fn read(&mut self, _uc: &mut Unicorn<()>, offset: u32) -> u32 {
        let spi = &mut self.inner;
        match offset {
            0x0000 => {
                spi.cr1
            }
            0x0008 => {
                // SR register
                // receive buffer not empty
                // transmit buffer empty
                0b11
            }
            0x000C => {
                // DR register
                if spi.bits16 {
                    if spi.rx.len() >= 2 {
                        ((spi.rx.pop_front().unwrap() as u32) << 8) |
                          spi.rx.pop_front().unwrap() as u32
                    } else {
                        0
                    }
                } else {
                    spi.rx.pop_front().unwrap_or(0) as u32
                }
            }
            _ => 0
        }
    }

    fn write(&mut self, _uc: &mut Unicorn<()>, offset: u32, value: u32) {
        match offset {
            0x0000 => {
                // CR1 register
                self.inner.bits16 = value & (1 << 11) != 0;
                self.inner.cr1 = value;
            }
            0x000C => {
                // DR register
                if self.inner.bits16 {
                    self.inner.tx.push_back((value >> 8) as u8);
                }
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

pub trait SpiDevice: Sized {
    fn name() -> Option<&'static str> { None }
    fn xfer(&mut self, spi: &mut SpiInner) -> Option<VecDeque<u8>>;
}

#[derive(Default)]
pub struct GenericSpiDevice;

impl SpiDevice for GenericSpiDevice {
    fn xfer(&mut self, spi: &mut SpiInner) -> Option<VecDeque<u8>> {
        debug!("{} tx={:x?}", spi.name, spi.tx);
        None
    }
}


/*
  pGVar1 = PTR_GPIOG_080295f4;
  HAL_GPIO_ResetPins(PTR_GPIOG_080295f4,p15);
  SpiTransfer(0x9f);
  iVar2 = SpiTransfer(0xff);
  iVar3 = SpiTransfer(0xff);
  uVar4 = SpiTransfer(0xff);
  HAL_GPIO_SetPins(pGVar1,p15);
  return iVar2 << 16 | iVar3 << 8 | uVar4;
*/
