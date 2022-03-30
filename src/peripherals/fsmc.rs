// SPDX-License-Identifier: GPL-3.0-or-later

use std::{cell::RefCell, rc::Rc};

use crate::{system::System, ext_devices::{ExtDevices, ExtDevice}};
use super::Peripheral;

pub struct Fsmc {
    banks: [Bank; 4],
}

impl Fsmc {
    pub fn new(name: &str, ext_devices: &ExtDevices) -> Option<Box<dyn Peripheral>> {
        if name.starts_with("FSMC") {
            let banks = [
                Bank::new(0, ext_devices),
                Bank::new(1, ext_devices),
                Bank::new(2, ext_devices),
                Bank::new(3, ext_devices),
            ];
            Some(Box::new(Self { banks }))
        } else {
            None
        }
    }

    fn access(offset: u32) -> Access {
        match offset {
            0x0000_0000..=0x0fff_ffff => Access::Data(0, offset),
            0x1000_0000..=0x1fff_ffff => Access::Data(1, offset - 0x1000_0000),
            0x2000_0000..=0x2fff_ffff => Access::Data(2, offset - 0x2000_0000),
            0x3000_0000..=0x3fff_ffff => Access::Data(3, offset - 0x3000_0000),
            0x4000_0000..=0x4fff_ffff => {
                match offset - 0x4000_0000 {
                    0x0000 => Access::Register(0, Reg::BCR),
                    0x0004 => Access::Register(0, Reg::BTR),
                    0x0008 => Access::Register(1, Reg::BCR),
                    0x000C => Access::Register(1, Reg::BTR),
                    0x0010 => Access::Register(2, Reg::BCR),
                    0x0014 => Access::Register(2, Reg::BTR),
                    0x0018 => Access::Register(3, Reg::BCR),
                    0x001C => Access::Register(3, Reg::BTR),
                    0x0060 => Access::Register(1, Reg::PCR),
                    0x0064 => Access::Register(1, Reg::SR),
                    0x0068 => Access::Register(1, Reg::PMEM),
                    0x006C => Access::Register(1, Reg::PATT),
                    0x0074 => Access::Register(1, Reg::ECCR),
                    0x0080 => Access::Register(2, Reg::PCR),
                    0x0084 => Access::Register(2, Reg::SR),
                    0x0088 => Access::Register(2, Reg::PMEM),
                    0x008C => Access::Register(2, Reg::PATT),
                    0x0094 => Access::Register(2, Reg::ECCR),
                    0x00A0 => Access::Register(3, Reg::PCR),
                    0x00A4 => Access::Register(3, Reg::SR),
                    0x00A8 => Access::Register(3, Reg::PMEM),
                    0x00AC => Access::Register(3, Reg::PATT),
                    0x00B0 => Access::Register(3, Reg::PIO),
                    0x0104 => Access::Register(0, Reg::BWTR),
                    0x010C => Access::Register(1, Reg::BWTR),
                    0x0114 => Access::Register(2, Reg::BWTR),
                    0x011C => Access::Register(3, Reg::BWTR),
                    _ => Access::Register(0, Reg::Invalid),
                }
            }
            _ => unreachable!()
        }
    }
}

impl Peripheral for Fsmc {
    fn read(&mut self, sys: &System, offset: u32) -> u32 {
        match Self::access(offset) {
            Access::Data(bank, offset) => self.banks[bank].read_data(sys, offset),
            Access::Register(bank, reg) => self.banks[bank].read_reg(sys, reg),
        }
    }

    fn write(&mut self, sys: &System, offset: u32, value: u32) {
        match Self::access(offset) {
            Access::Data(bank, offset) => self.banks[bank].write_data(sys, offset, value),
            Access::Register(bank, reg) => self.banks[bank].write_reg(sys, reg, value),
        }
    }
}

pub trait FsmcDevice {
    fn name(&self, fsmc_bank_name: &str) -> String;
    fn read_data(&mut self, bank: &mut Bank, offset: u32) -> u32;
    fn write_data(&mut self, bank: &mut Bank, offset: u32, value: u32);
}

pub struct Bank {
    pub name: String,
    ext_device: Option<Rc<RefCell<dyn ExtDevice<u32, u32>>>>,
}

impl Bank {
    pub fn new(bank: usize, ext_devices: &ExtDevices) -> Self {
        let name = format!("FSMC.BANK{}", bank+1);

        let ext_device = ext_devices.find_mem_device(&name);
        let name = ext_device.as_ref()
            .map(|d| d.borrow_mut().connect_peripheral(&name))
            .unwrap_or(name);

        Self { name, ext_device }
    }

    fn read_data(&mut self, sys: &System, offset: u32) -> u32 {
        let v = self.ext_device.as_ref().map(|d|
            d.borrow_mut().read(sys, offset)
        ).unwrap_or_default();

        trace!("{} data read at offset=0x{:08x} value=0x{:08x}", self.name, offset, v);

        v
    }

    fn write_data(&mut self, sys: &System, offset: u32, value: u32) {
        self.ext_device.as_ref().map(|d|
            d.borrow_mut().write(sys, offset, value)
        );

        trace!("{} data write at offset=0x{:08x} value=0x{:08x}", self.name, offset, value);
    }

    fn read_reg(&mut self, _sys: &System, reg: Reg) -> u32 {
        trace!("{} read reg={:?}", self.name, reg);
        0
    }

    fn write_reg(&mut self, _sys: &System, reg: Reg, _value: u32) {
        trace!("{} write reg={:?}", self.name, reg);
    }
}

enum Access {
    Data(usize, u32),
    Register(usize, Reg),
}

#[derive(Debug)]
enum Reg {
    BCR,
    BTR,
    PMEM,
    PATT,
    ECCR,
    PCR,
    SR,
    BWTR,
    PIO,
    Invalid,
}
