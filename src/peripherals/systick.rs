use svd_parser::svd::{MaybeArray, RegisterInfo};
use unicorn_engine::Unicorn;

use super::Peripheral;

#[derive(Default)]
pub struct SysTick {
    reload: u32,
    val_toggle: bool,
}

impl SysTick {
    pub fn use_peripheral(name: &str) -> bool {
        name == "STK"
    }

    pub fn new(_name: String, _registers: &[MaybeArray<RegisterInfo>]) -> Self {
        Self::default()
    }
}


impl Peripheral for SysTick {
    fn read(&mut self, _uc: &mut Unicorn<()>, offset: u32, _size: usize) -> u32 {
        if offset == 0x0004 {
            self.reload
        } else if offset == 0x0008 {
            self.val_toggle = !self.val_toggle;
            if self.val_toggle { 0 } else { self.reload/2 }
        } else {
            0
        }
    }

    fn write(&mut self, _uc: &mut Unicorn<()>, offset: u32, _size: usize, value: u32) {
        if offset == 0x0004 {
            // LOAD register
            self.reload = value
        }
    }
}
