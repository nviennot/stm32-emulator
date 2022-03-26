// SPDX-License-Identifier: GPL-3.0-or-later

use svd_parser::svd::{MaybeArray, RegisterInfo};
use unicorn_engine::Unicorn;

use super::Peripheral;

#[derive(Default)]
pub struct Gpio {
    block: char,
    mode: u32,
    otype: u32,
    ospeed: u32,
    pupd: u32,
    id: u32,
    od: u32,
    bsr: u32,
    lck: u32,
    afrl: u32,
    afrh: u32,
}

impl Gpio {
    pub fn use_peripheral(name: &str) -> bool {
        name.starts_with("GPIO")
    }

    pub fn new(name: String, _registers: &[MaybeArray<RegisterInfo>]) -> Self {
        let block = name.strip_prefix("GPIO").unwrap().chars().next().unwrap();
        Self { block, ..Self::default() }
    }

    // f(port, values)
    fn iter_port_reg_changes(old_value: u32, new_value: u32, stride: u8, mut f: impl FnMut(u8, u8)) {
        let mut changes = old_value ^ new_value;
        let stride_mask = 0xFF >> (8 - stride);
        while changes != 0 {
            let right_most_bit = changes.trailing_zeros() as u8;
            let port = right_most_bit / stride;
            if port <= 16 {
                let v = (new_value >> (port*stride)) as u8 & stride_mask;
                f(port, v);
            }
            changes &= !(stride_mask as u32) << (port*stride);
        }
    }

    fn port_str(&self, port: u8) -> String {
        format!("GPIO P{}{}", self.block, port)
    }
}

impl Peripheral for Gpio {
    fn read(&mut self, _uc: &mut Unicorn<()>, offset: u32) -> u32 {
        match offset {
            0x0000 => self.mode,
            0x0004 => self.otype,
            0x0008 => self.ospeed,
            0x000C => self.pupd,
            0x0010 => self.id,
            0x0014 => self.od,
            0x0018 => self.bsr,
            0x001C => self.lck,
            0x0020 => self.afrl,
            0x0024 => self.afrh,
            _ => {
                warn!("GPIO invalid offset=0x{:08x}", offset);
                0
            }
        }
    }

    fn write(&mut self, _uc: &mut Unicorn<()>, offset: u32, value: u32) {
        match offset {
            0x0000 => {
                Self::iter_port_reg_changes(self.mode, value, 2, |port, v| {
                    let config = match v {
                        0b00 => "input",
                        0b01 => "output",
                        0b10 => "alternate",
                        0b11 => "analog",
                        _ => unreachable!(),
                    };
                    debug!("{} configured as {}", self.port_str(port), config);
                });
                self.mode = value;
            }
            0x0004 => {
                Self::iter_port_reg_changes(self.otype, value, 1, |port, v| {
                    let config = match v {
                        0b0 => "push-pull",
                        0b1 => "open-drain",
                        _ => unreachable!(),
                    };
                    debug!("{} configured as {}", self.port_str(port), config);
                });
                self.otype = value;
            }
            0x0008 => {
                Self::iter_port_reg_changes(self.ospeed, value, 2, |port, v| {
                    let config = match v {
                        0b00 => "low speed",
                        0b01 => "medium speed",
                        0b10 => "high speed",
                        0b11 => "very high speed",
                        _ => unreachable!(),
                    };
                    debug!("{} configured as {}", self.port_str(port), config);
                });
                self.ospeed = value;
            }
            0x000C => {
                self.pupd = value;
            }
            0x0010 => {
                self.id = value;
            }
            0x0014 => {
                self.od = value;
            }
            0x0018 => {
                self.bsr = value;
            }
            0x001C => {
                self.lck = value;
            }
            0x0020 => {
                self.afrl = value;
            }
            0x0024 => {
                self.afrh = value;
            }
            _ => {
                warn!("GPIO invalid offset=0x{:08x}", offset);
            }
        }
    }
}
