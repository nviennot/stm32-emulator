
use svd_parser::svd::{MaybeArray, RegisterInfo};
use unicorn_engine::Unicorn;

use super::Peripheral;

pub struct Spi {
}

impl Spi {
    pub fn use_peripheral(name: &str) -> bool {
        name.starts_with("SPI")
    }

    pub fn new(_name: String, _registers: &[MaybeArray<RegisterInfo>]) -> Self {
        Self {}
    }
}


impl Peripheral for Spi {
    fn read(&mut self, _uc: &mut Unicorn<()>, offset: u32, _size: usize) -> u32 {
        if offset == 0x0008 {
            // SR register
            // receive buffer not empty
            // transmit buffer empty
            0b11
        } else {
            0
        }
    }

    fn write(&mut self, _uc: &mut Unicorn<()>, _offset: u32, _size: usize, _value: u32) {
    }
}
