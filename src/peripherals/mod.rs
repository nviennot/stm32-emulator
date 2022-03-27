// SPDX-License-Identifier: GPL-3.0-or-later

mod rcc;
pub mod spi;
mod systick;
mod gpio;
mod dma;

use rcc::*;
use spi::*;
use systick::*;
use gpio::*;
use dma::*;

use std::{collections::BTreeMap, cell::RefCell};
use svd_parser::svd::RegisterInfo;
use unicorn_engine::Unicorn;

use crate::devices::Devices;

pub struct Peripherals {
    debug_peripherals: Vec<PeripheralSlot<GenericPeripheral>>,
    peripherals: Vec<PeripheralSlot<RefCell<Box<dyn Peripheral>>>>,
}

pub struct PeripheralSlot<T> {
    pub start: u32,
    pub end: u32,
    pub peripheral: T,
}

impl Peripherals {
    // start - end regions
    pub const MEMORY_MAPS: [(u32, u32); 3] = [
        (0x4000_0000, 0x8000_0000),
        (0xA000_0000, 0xB000_0000), // FSMC
        (0xE000_0000, 0xE100_0000),
    ];

    pub fn new() -> Self {
        let debug_peripherals = vec![];
        let peripherals = vec![];
        Self { debug_peripherals, peripherals }
    }

    pub fn register_peripheral(&mut self,
        name: String,
        base: u32,
        registers: &[RegisterInfo],
        devices: &mut Devices,
    ) {
        let p = GenericPeripheral::new(name.clone(), registers);

        trace!("Peripheral base=0x{:08x} size=0x{:08} name={}", base, p.size(), p.name());

        if let Some(last_p) = self.debug_peripherals.last() {
            assert!(last_p.start < base, "Register blocks must be sorted");
            assert!(last_p.end < base, "Overlapping register blocks between {} and {}", last_p.peripheral.name(), p.name());
        }

        let start = base;
        let end = base+p.size();

        self.debug_peripherals.push(PeripheralSlot { start, end, peripheral: p });

        let p = None
            .or_else(|| SysTick::new(&name))
            .or_else(||    Gpio::new(&name))
            .or_else(||     Rcc::new(&name))
            .or_else(||     Dma::new(&name))
            .or_else(||     Spi::new(&name, devices))
        ;

        if let Some(p) = p {
            self.peripherals.push(PeripheralSlot { start, end, peripheral: RefCell::new(p) });
        }
    }

    pub fn finish_registration(&mut self) {
    }

    pub fn get_peripheral<T>(peripherals: &Vec<PeripheralSlot<T>>, addr: u32) -> Option<&PeripheralSlot<T>> {
        let index = peripherals.binary_search_by_key(&addr, |p| p.start)
            .map_or_else(|e| e.checked_sub(1), |v| Some(v));

        index.map(|i| peripherals.get(i).filter(|p| addr <= p.end)).flatten()
    }

    pub fn addr_desc(&self, addr: u32) -> String {
        if let Some(p) = Self::get_peripheral(&self.debug_peripherals, addr) {
            format!("addr=0x{:08x} peri={} {}", addr, p.peripheral.name, p.peripheral.reg_name(addr - p.start))
        } else {
            format!("addr=0x{:08x} peri=????", addr)
        }
    }

    pub fn read(&mut self, uc: &mut Unicorn<()>, addr: u32, size: u8) -> u32 {
        if (0x4200_0000..0x4400_0000).contains(&addr) {
            // Bit-banding
            let bit_number = (addr % 32) / 4;
            let addr = 0x4000_0000 + (addr - 0x4200_0000)/32;
            return (self.read(uc, addr, 1) >> bit_number) & 1
        }

        // Reduce the reads to 4 byte alignements
        let byte_offset = (addr % 4) as u8;
        assert!(byte_offset + size <= 4);
        let addr = addr - byte_offset as u32;

        let value = if let Some(p) = Self::get_peripheral(&self.peripherals, addr) {
            p.peripheral.borrow_mut().read(&self, uc, addr - p.start) << (8*byte_offset)
        } else {
            0
        };

        if crate::verbose() >= 3 {
            trace!("read:  {} read=0x{:08x}", self.addr_desc(addr), value);
        }

        value
    }

    pub fn write(&mut self, uc: &mut Unicorn<()>, addr: u32, size: u8, mut value: u32) {
        if (0x4200_0000..0x4400_0000).contains(&addr) {
            // Bit-banding
            let bit_number = (addr % 32) / 4;
            let addr = 0x4000_0000 + (addr - 0x4200_0000)/32;
            let mut v = self.read(uc, addr, 1);
            v &= 1 << bit_number;
            v |= (value & 1) << bit_number;
            return self.write(uc, addr, 1, v);
        }

        // Reduce the writes to 4 byte alignements
        let byte_offset = (addr % 4) as u8;
        assert!(byte_offset + size <= 4);
        let addr = addr - byte_offset as u32;

        if byte_offset != 0 {
            let v = self.read(uc, addr, 4);
            value = (value << 8*byte_offset) | (v & (0xFFFF_FFFF >> (32-8*byte_offset)));
        }

        if crate::verbose() >= 3 {
            trace!("write: {} write=0x{:08x}", self.addr_desc(addr), value);
        }

        if let Some(p) = Self::get_peripheral(&self.peripherals, addr) {
            p.peripheral.borrow_mut().write(self, uc, addr - p.start, value)
        }
    }
}

pub trait Peripheral {
    fn read(&mut self, perifs: &Peripherals, uc: &mut Unicorn<()>, offset: u32) -> u32;
    fn write(&mut self, perifs: &Peripherals, uc: &mut Unicorn<()>, offset: u32, value: u32);
}

struct GenericPeripheral {
    pub name: String,
    // offset -> name
    pub registers: BTreeMap<u32, RegisterInfo>,
}

impl GenericPeripheral {
    pub fn new(name: String, registers: &[RegisterInfo]) -> Self {
        let registers = registers.iter()
            .map(|r| (r.address_offset, r.clone()))
            .collect();

        Self { name, registers }
    }

    pub fn reg_name(&self, offset: u32) -> String {
        assert!(offset % 4 == 0);
        let reg = self.registers.get(&offset);
        reg.map(|r| &r.name)
            .map(|r| format!("offset=0x{:04x} reg={}", offset, r))
            .unwrap_or_else(|| format!("offset=0x{:04x} reg=????", offset))
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn size(&self) -> u32 {
        self.registers
            .keys()
            .cloned()
            .max()
            .unwrap_or(0) + 4
    }
}
