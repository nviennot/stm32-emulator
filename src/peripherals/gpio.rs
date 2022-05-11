// SPDX-License-Identifier: GPL-3.0-or-later

use crate::system::System;
use super::Peripheral;

use regex::Regex;

const NUM_PORTS: usize = 11;

#[derive(Clone, Copy)]
pub struct Pin {
    port: u8,
    pin: u8,
}

impl Pin {
    pub fn from_str(name: &str) -> Self {
        let name = name.to_uppercase();
        let re = Regex::new(r"^P?([A-Z])(\d+)$").unwrap();
        let captures = re.captures(&name).expect("Pin name invalid");
        let port = captures.get(1).unwrap().as_str().chars().next().unwrap();
        let port = GpioPorts::port_index(port);
        let pin = captures.get(2).unwrap().as_str().parse().unwrap();
        assert!(pin < 16);
        Self { port, pin }
    }
}

#[derive(Default)]
pub struct GpioPorts {
    read_callbacks: [Vec<(u8, Box<dyn FnMut(&System) -> bool>)>; NUM_PORTS],
    write_callbacks: [Vec<(u8, Box<dyn FnMut(&System, bool)>)>; NUM_PORTS],
}

impl GpioPorts {
    pub fn port_index(letter: char) -> u8 {
        match letter {
            'A'..='K' => letter as u8 - 'A' as u8,
            _ => panic!("Invalid GPIO port {}", letter),
        }
    }

    pub fn add_read_callback(&mut self, pin: Pin, cb: impl FnMut(&System) -> bool + 'static) {
        self.read_callbacks[pin.port as usize].push((pin.pin, Box::new(cb)));
    }

    pub fn add_write_callback(&mut self, pin: Pin, cb: impl FnMut(&System, bool) + 'static) {
        self.write_callbacks[pin.port as usize].push((pin.pin, Box::new(cb)));
    }

    pub fn read_port(&mut self, sys: &System, port: u8) -> u16 {
        let mut v = 0;
        for (pin, cb) in &mut self.read_callbacks[port as usize] {
            if cb(sys) {
                v |= 1 << *pin;
            }
        }
        v
    }

    pub fn write_port(&mut self, sys: &System, port: u8, pin: u8, value: bool) {
        for (pin_cb, cb) in &mut self.write_callbacks[port as usize] {
            if *pin_cb == pin {
                cb(sys, value);
            }
        }
    }
}

#[derive(Default)]
pub struct Gpio {
    port_letter: char,
    port: u8,

    mode: u32,
    otype: u32,
    ospeed: u32,
    pupd: u32,
    od: u32,
    lck: u32,
    afrl: u32,
    afrh: u32,
}

impl Gpio {
    pub fn new(name: &str) -> Option<Box<dyn Peripheral>> {
        if let Some(block) = name.strip_prefix("GPIO") {
            let port_letter = block.chars().next().unwrap();
            let port = GpioPorts::port_index(port_letter);
            Some(Box::new(Self { port_letter, port, ..Self::default() }))
        } else {
            None
        }
    }

    // f(port, values)
    fn iter_port_reg_changes(old_value: u32, new_value: u32, stride: u8, mut f: impl FnMut(u8, u8)) {
        let mut changes = old_value ^ new_value;
        let stride_mask = 0xFF >> (8 - stride);
        while changes != 0 {
            let right_most_bit = changes.trailing_zeros() as u8;
            let pin = right_most_bit / stride;
            if pin <= 16 {
                let v = (new_value >> (pin*stride)) as u8 & stride_mask;
                f(pin, v);
            }
            changes &= !(stride_mask as u32) << (pin*stride);
        }
    }

    fn port_str(&self, pin: u8) -> String {
        format!("GPIO{} P{}{}", self.port_letter, self.port_letter, pin)
    }
}

impl Peripheral for Gpio {
    fn read(&mut self, sys: &System, offset: u32) -> u32 {
        match offset {
            0x0000 => self.mode,
            0x0004 => self.otype,
            0x0008 => self.ospeed,
            0x000C => self.pupd,
            0x0010 => {
                let v = sys.p.gpio.borrow_mut().read_port(sys, self.port);
                trace!("GPIO{} read v=0x{:04x}", self.port_letter, v);
                v as u32
            }
            0x0014 => self.od,
            0x0018 => 0, // bsr
            0x001C => self.lck,
            0x0020 => self.afrl,
            0x0024 => self.afrh,
            _ => {
                warn!("GPIO invalid offset=0x{:08x}", offset);
                0
            }
        }
    }

    fn write(&mut self, sys: &System, offset: u32, value: u32) {
        match offset {
            0x0000 => {
                Self::iter_port_reg_changes(self.mode, value, 2, |pin, v| {
                    let config = match v {
                        0b00 => "input",
                        0b01 => "output",
                        0b10 => "alternate",
                        0b11 => "analog",
                        _ => unreachable!(),
                    };
                    trace!("{} mode={}", self.port_str(pin), config);
                });
                self.mode = value;
            }
            0x0004 => {
                Self::iter_port_reg_changes(self.otype, value, 1, |pin, v| {
                    let config = match v {
                        0b0 => "push-pull",
                        0b1 => "open-drain",
                        _ => unreachable!(),
                    };
                    trace!("{} output_cfg={}", self.port_str(pin), config);
                });
                self.otype = value;
            }
            0x0008 => {
                Self::iter_port_reg_changes(self.ospeed, value, 2, |pin, v| {
                    let config = match v {
                        0b00 => "low",
                        0b01 => "medium",
                        0b10 => "high",
                        0b11 => "very-high",
                        _ => unreachable!(),
                    };
                    trace!("{} speed={}", self.port_str(pin), config);
                });
                self.ospeed = value;
            }
            0x000C => {
                Self::iter_port_reg_changes(self.pupd, value, 2, |pin, v| {
                    let config = match v {
                        0b00 => "regular",
                        0b01 => "pull-up",
                        0b10 => "pull-down",
                        0b11 => "reserved",
                        _ => unreachable!(),
                    };
                    trace!("{} input_cfg={}", self.port_str(pin), config);
                });
                self.pupd = value;
            }
            0x0010 => {
                // input data register. read-only
            }
            0x0014 => {
                let mut gpio = sys.p.gpio.borrow_mut();
                Self::iter_port_reg_changes(self.od, value, 1, |pin, v| {
                    gpio.write_port(sys, self.port, pin, v != 0);
                    trace!("{} output={}", self.port_str(pin), v);
                });
                self.od = value;
            }
            0x0018 => {
                let reset = value >> 16;
                let set = value & 0xFFFF;
                let mut gpio = sys.p.gpio.borrow_mut();

                Self::iter_port_reg_changes(0, set, 1, |pin, _| {
                    gpio.write_port(sys, self.port, pin, true);
                    trace!("{} output=1", self.port_str(pin));
                });

                Self::iter_port_reg_changes(0, reset, 1, |pin, _| {
                    gpio.write_port(sys, self.port, pin, false);
                    trace!("{} output=0", self.port_str(pin));
                });

                self.od &= !reset;
                self.od |= set;
            }
            0x001C => {
                trace!("GPIO{} port locked", self.port_letter);
                self.lck = value;
            }
            0x0020 => {
                Self::iter_port_reg_changes(self.afrl, value, 4, |pin, v| {
                    trace!("{} alternate_cfg=AF{}", self.port_str(pin), v);
                });
                self.afrl = value;
            }
            0x0024 => {
                Self::iter_port_reg_changes(self.afrh, value, 4, |pin, v| {
                    trace!("{} alternate_cfg=AF{}", self.port_str(pin+8), v);
                });
                self.afrh = value;
            }
            _ => {
                warn!("GPIO invalid offset=0x{:08x}", offset);
            }
        }
    }
}
