// SPDX-License-Identifier: GPL-3.0-or-later

use std::rc::Rc;
use std::{collections::VecDeque, cell::RefCell};
use std::convert::TryFrom;

use anyhow::Result;
use serde::Deserialize;

use crate::framebuffers::{Framebuffer, Framebuffers, RGB565};
use crate::peripherals::gpio::{GpioPorts, Pin};
use crate::system::System;

use super::ExtDevice;

// Implements a ADS7846 controller

#[derive(Debug, Deserialize, Default)]
pub struct TouchscreenConfig {
    pub peripheral: String,
    pub framebuffer: String,
    pub flip_x: Option<bool>,
    pub flip_y: Option<bool>,
    pub swap_x_y: Option<bool>,
    pub touch_detected_pin: Option<String>,
    pub scale_down: Option<u32>,
}

pub struct Touchscreen {
    pub config: TouchscreenConfig,
    name: String,

    framebuffer: Rc<RefCell<dyn Framebuffer<RGB565>>>,
    reply: Option<VecDeque<u8>>,
}

impl Touchscreen {
    pub fn new(config: TouchscreenConfig, gpio: &mut GpioPorts, framebuffers: &Framebuffers) -> Result<Self> {
        let framebuffer = framebuffers.get(&config.framebuffer)?;

        if let Some(ref touch_detected_pin) = config.touch_detected_pin {
            let touch_detected_pin = Pin::from_str(touch_detected_pin);
            let framebuffer = framebuffer.clone();
            gpio.add_read_callback(touch_detected_pin, move |_sys| {
                framebuffer.borrow().get_touch_position().is_none()
            });
        }

        Ok(Self {
            config,
            name: "".to_string(), // filled up in connect_periperhal()
            framebuffer,
            reply: None,
        })
    }
}

impl ExtDevice<(), u8> for Touchscreen {
    fn connect_peripheral(&mut self, peri_name: &str) -> String {
        self.name = format!("{} touchscreen", peri_name);
        self.name.clone()
    }

    fn read(&mut self, _sys: &System, _addr: ()) -> u8 {
        if let Some(reply) = self.reply.as_mut() {
            reply.pop_front().unwrap_or_default()
        } else {
            0
        }
    }

    fn write(&mut self, _sys: &System, _addr: (), v: u8) {
        if let Some(cmd) = Command::try_from(v).ok() {
            let fb = self.framebuffer.borrow();
            const MAX: u32 = 0xfff;
            if let Some(pos) = fb.get_touch_position() {
                let op = match (self.config.swap_x_y, cmd.op) {
                    (Some(true), Operation::MeasureX) => Operation::MeasureY,
                    (Some(true), Operation::MeasureY) => Operation::MeasureX,
                    _ => cmd.op,
                };

                let v = match op {
                    Operation::MeasureX => (pos.0 as u32 * MAX) / fb.get_config().width as u32,
                    Operation::MeasureY => (pos.1 as u32 * MAX) / fb.get_config().height as u32,
                    Operation::MeasureZ1 => 10,
                    Operation::MeasureZ2 => 10,
                };

                let v = match (op, self.config.flip_x, self.config.flip_y) {
                    (Operation::MeasureX, Some(true), _) => (MAX - v),
                    (Operation::MeasureY, _, Some(true)) => (MAX - v),
                    _ => v,
                };

                // Not sure why we need this
                let v = if let Some(scale_down) = self.config.scale_down {
                    v / scale_down
                } else {
                    v
                };

                // We don't care if we are doing a 12bit or 8bit convertion as MSB comes first.
                // 0000AABB CCDDEEFF -> AABBCCDD EEFF0000
                debug!("{} cmd={:?} reply={:04x}", self.name, cmd, v);
                self.reply = Some(vec![(v >> 4) as u8, (v << 4) as u8].into());
            } else {
                // all zeros
                self.reply = None;
            }

        }
    }
}

#[derive(Debug, Clone, Copy, num_enum::TryFromPrimitive)]
#[repr(u8)]
enum Operation {
    MeasureX = 0b101,
    MeasureY = 0b001,
    MeasureZ1 = 0b011,
    MeasureZ2 = 0b100,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, num_enum::TryFromPrimitive)]
#[repr(u8)]
enum Mode {
    M12Bits = 0,
    M8Bits = 1,
}

#[derive(Debug, Clone, Copy, num_enum::TryFromPrimitive)]
#[repr(u8)]
enum Power {
    LowPower = 0b00,
    RefOffAdcOn = 0b01,
    RefOnAdcOff = 0b10,
    AlwaysOn = 0b11,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
struct Command {
    pub op: Operation,
    pub mode: Mode,
    pub differential: bool,
    pub power: Power,
}

impl std::convert::TryFrom<u8> for Command {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        // bits:
        // S A2 A1 A0 _ Mode SER PD1 PD0
        // Check for start bit
        if value & 0b1000_0000 != 0 {
            let op = (value >> 4) & 0b111;
            let op = Operation::try_from(op).expect("touchscreen operation unknown");

            let mode = (value >> 3) & 1;
            let mode = Mode::try_from(mode).unwrap();

            let differential = (value >> 2) & 1 != 0;

            let power = value & 0b11;
            let power = Power::try_from(power).unwrap();

            Ok(Self { op, mode, differential, power })
        } else {
            Err(())
        }
    }
}
