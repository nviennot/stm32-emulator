// SPDX-License-Identifier: GPL-3.0-or-later

use crate::system::System;
use super::Peripheral;

#[derive(Default)]
pub struct Fsmc {
    banks: [Bank; 4]
}

impl Fsmc {
    pub fn new(name: &str) -> Option<Box<dyn Peripheral>> {
        if name == "FSMC" {
            Some(Box::new(Fsmc::default()))
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
            Access::Data(bank, offset) => self.banks[bank].read_data(bank, sys, offset),
            Access::Register(bank, reg) => self.banks[bank].read_reg(bank, sys, reg),
        }
    }

    fn write(&mut self, sys: &System, offset: u32, value: u32) {
        match Self::access(offset) {
            Access::Data(bank, offset) => self.banks[bank].write_data(bank, sys, offset, value),
            Access::Register(bank, reg) => self.banks[bank].write_reg(bank, sys, reg, value),
        }
    }
}


#[derive(Default)]
pub struct Bank {
}

impl Bank {
    fn cmd_pin(offset: u32) -> &'static str {
        if offset & (1 << (12+1)) != 0 {
            "data"
        } else {
            "cmd"
        }
    }

    fn read_data(&mut self, bank: usize, _sys: &System, offset: u32) -> u32 {
        trace!("FSMC bank={} data read at offset=0x{:08x}", bank+1, offset);
        debug!("FSMC display READ {}", Self::cmd_pin(offset));
        0
    }

    fn write_data(&mut self, bank: usize, _sys: &System, offset: u32, value: u32) {
        trace!("FSMC bank={} data write at offset=0x{:08x}", bank+1, offset);
        debug!("FSMC display WRITE {} value=0x{:04x}", Self::cmd_pin(offset), value as u16);
    }

    fn read_reg(&mut self, bank: usize, _sys: &System, reg: Reg) -> u32 {
        trace!("FSMC bank={} read reg={:?}", bank+1, reg);
        0
    }

    fn write_reg(&mut self, bank: usize, _sys: &System, reg: Reg, _value: u32) {
        trace!("FSMC bank={} write reg={:?}", bank+1, reg);
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
