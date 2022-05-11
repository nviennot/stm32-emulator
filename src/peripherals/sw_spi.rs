// SPDX-License-Identifier: GPL-3.0-or-later

use std::rc::Rc;
use std::{cell::RefCell};

use serde::Deserialize;

use crate::ext_devices::{ExtDevice, ExtDevices};
use crate::peripherals::gpio::{Pin, GpioPorts};
use crate::system::System;

#[derive(Debug, Deserialize, Default)]
pub struct SoftwareSpiConfig {
    pub name: String,
    pub cs: Option<String>,
    pub clk: String,
    pub miso: String,
    pub mosi: String,
    // TODO clk polarity
}

#[derive(Default)]
pub struct SoftwareSpi {
    pub config: SoftwareSpiConfig,
    name: String,

    data_mosi: u8,
    data_miso: u8,
    bit_index: u8,

    cs: bool,
    clk: bool,
    mosi: bool,
    miso: bool,

    ext_device: Option<Rc<RefCell<dyn ExtDevice<(), u8>>>>,
}

impl SoftwareSpi {
    pub fn register(config: SoftwareSpiConfig, gpio: &mut GpioPorts, ext_devices: &ExtDevices) {
        let cs = config.cs.as_ref().map(|s| Pin::from_str(s));
        let clk = Pin::from_str(&config.clk);
        let miso = Pin::from_str(&config.miso);
        let mosi = Pin::from_str(&config.mosi);

        let ext_device = ext_devices.find_serial_device(&config.name);
        let name = ext_device.as_ref()
            .map(|d| d.borrow_mut().connect_peripheral(&config.name))
            .unwrap_or_else(|| config.name.to_string());

        let self_ = Rc::new(RefCell::new(Self { config, name, ext_device, ..Default::default() }));

        if let Some(cs) = cs {
            let s = self_.clone();
            gpio.add_write_callback(cs, move |sys, v| { s.borrow_mut().write_cs(sys, v) });
        }

        let s = self_.clone();
        gpio.add_write_callback(clk, move |sys, v| { s.borrow_mut().write_clk(sys, v) });

        let s = self_.clone();
        gpio.add_read_callback(miso, move |sys| { s.borrow_mut().read_miso(sys) });

        let s = self_.clone();
        gpio.add_write_callback(mosi, move |sys, v| { s.borrow_mut().write_mosi(sys, v) });
    }

    pub fn write_cs(&mut self, _sys: &System, value: bool) {
        // edge down
        if self.cs && !value {
            self.data_mosi = 0;
            self.data_miso = 0;
            self.bit_index = 0;

            self.clk = false;
            self.mosi = false;
            self.miso = false;
        }
        self.cs = value;
    }

    pub fn write_clk(&mut self, sys: &System, value: bool) {
        if self.cs { return; }

        // clock rise
        if !self.clk && value {
            self.miso = self.data_miso & 0x80 != 0;
            self.data_miso <<= 1;

            self.data_mosi <<= 1;
            if self.mosi {
                self.data_mosi |= 1;
            }

            self.bit_index += 1;
            if self.bit_index == 8 {
                self.bit_index = 0;
                self.data_miso = self.xfer(sys, self.data_mosi);
            }
        }

        self.clk = value;
    }

    pub fn read_miso(&mut self, _sys: &System) -> bool {
        if self.cs { return false; }
        self.miso
    }

    pub fn write_mosi(&mut self, _sys: &System, value: bool) {
        if self.cs { return; }
        self.mosi = value;
    }

    fn xfer(&mut self, sys: &System, mosi: u8) -> u8 {
        trace!("{} write={:02x}", self.name, mosi);
        let miso = if let Some(ref d) = self.ext_device {
            let mut d = d.borrow_mut();
            d.write(sys, (), mosi);
            d.read(sys, ())
        } else {
            0
        };
        trace!("{} read={:02x}", self.name, miso);
        miso
    }
}
