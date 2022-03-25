mod rcc;
use rcc::*;

use unicorn_engine::Unicorn;

pub struct Peripherals {

}

impl Peripherals {
    // start - end regions
    pub const MEMORY_MAPS: [(u32, u32); 2] = [
        (0x4000_0000, 0x8000_0000),
        (0xE000_0000, 0xE100_0000),
    ];

    pub fn new() -> Self {
        Self {}
    }

    pub fn read(&mut self, _uc: &mut Unicorn<()>, addr: u32) -> u32 {
        info!("read:  addr=0x{:08x}", addr);
        0
    }

    pub fn write(&mut self, _uc: &mut Unicorn<()>, addr: u32, value: u32) {
        info!("write: addr=0x{:08x}, value=0x{:08x}", addr, value);
    }
}
