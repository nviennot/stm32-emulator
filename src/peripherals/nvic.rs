// SPDX-License-Identifier: GPL-3.0-or-later

use std::{rc::Rc, cell::RefCell, sync::atomic::Ordering};

use unicorn_engine::RegisterARM;

use crate::system::System;
use super::Peripheral;

#[derive(Default)]
pub struct Nvic {
    pub systick_period: Option<u32>,
    pub last_systick_trigger: u64,

    // 128 different interrupts. Good enough for now
    // pending: u128,
    in_interrupt: bool,
}

pub mod irq {
    pub const SYSTICK: i32 = -1;
}

impl Nvic {
    pub fn run_pending_interrupts(&mut self, sys: &System, vector_table_addr: u32) {
        // Interrupt disabled?
        let primask = sys.uc.borrow().reg_read(RegisterARM::PRIMASK).unwrap();
        if primask != 0 || self.in_interrupt {
            return;
        }

        if let Some(systick_period) = self.systick_period {
            let n = crate::emulator::NUM_INSTRUCTIONS.load(Ordering::Relaxed);
            if n + systick_period as u64 > self.last_systick_trigger {
                self.last_systick_trigger = n;
                self.run_interrupt(sys, vector_table_addr, irq::SYSTICK);
            }
        }
    }

    fn read_vector_addr(sys: &System, vector_table_addr: u32, irq: i32) -> u32 {
        let vaddr = vector_table_addr + 4*(16 + irq) as u32;

        let mut vector = [0,0,0,0];
        sys.uc.borrow().mem_read(vaddr as u64, &mut vector).unwrap();
        u32::from_le_bytes(vector)
    }

    fn run_interrupt(&mut self, sys: &System, vector_table_addr: u32, irq: i32) {
        let vector = Self::read_vector_addr(sys, vector_table_addr, irq);

        trace!("Running interrupt n={}, vector={:#08x}", irq, vector);
        Self::push_regs(sys);

        let mut uc = sys.uc.borrow_mut();

        uc.reg_write(RegisterARM::IPSR, irq as u64).unwrap();
        uc.reg_write(RegisterARM::PC, vector as u64).unwrap();
        // This value means return from interrupt.
        uc.reg_write(RegisterARM::LR, 0xFFFF_FFFD).unwrap();

        self.in_interrupt = true;
    }

    pub fn return_from_interrupt(&mut self, sys: &System) {
        trace!("Return from interrupt");
        Self::pop_regs(sys);
        self.in_interrupt = false;
    }

    const CONTEXT_REGS: [RegisterARM; 25] = [
        RegisterARM::FPSCR,
        RegisterARM::S15,
        RegisterARM::S14,
        RegisterARM::S13,
        RegisterARM::S12,
        RegisterARM::S11,
        RegisterARM::S10,
        RegisterARM::S9,
        RegisterARM::S8,
        RegisterARM::S7,
        RegisterARM::S6,
        RegisterARM::S5,
        RegisterARM::S4,
        RegisterARM::S3,
        RegisterARM::S2,
        RegisterARM::S1,
        RegisterARM::S0,
        RegisterARM::XPSR,
        RegisterARM::PC,
        RegisterARM::LR,
        RegisterARM::R12,
        RegisterARM::R3,
        RegisterARM::R2,
        RegisterARM::R1,
        RegisterARM::R0,
    ];

    fn push_regs(sys: &System) {
        let mut uc = sys.uc.borrow_mut();
        let mut sp = uc.reg_read(RegisterARM::SP).unwrap();
        for reg in Self::CONTEXT_REGS {
            let v = uc.reg_read(reg).unwrap() as u32;
            sp -= 4;
            uc.mem_write(sp, &v.to_le_bytes()).expect("Invalid SP pointer during interrupt");
        }
        uc.reg_write(RegisterARM::SP, sp).unwrap();
    }

    fn pop_regs(sys: &System) {
        let mut uc = sys.uc.borrow_mut();
        let mut sp = uc.reg_read(RegisterARM::SP).unwrap();
        for reg in Self::CONTEXT_REGS.iter().rev() {
            let mut v = [0,0,0,0];
            uc.mem_read(sp, &mut v).expect("Invalid SP pointer during interrupt return");
            let v = u32::from_le_bytes(v);
            sp += 4;
            uc.reg_write(*reg, v as u64).unwrap();
        }
        uc.reg_write(RegisterARM::SP, sp).unwrap();
    }
}

impl Peripheral for Nvic {
    fn read(&mut self, _sys: &System, _offset: u32) -> u32 {
        0
    }

    fn write(&mut self, _sys: &System, _offset: u32, _value: u32) {
    }
}

/// The next part is glue. Maybe we could have a better architecture.

pub struct NvicWrapper(Rc<RefCell<Nvic>>);

impl NvicWrapper {
    pub fn new(name: &str, nvic: &Rc<RefCell<Nvic>>) -> Option<Box<dyn Peripheral>> {
        if name == "NVIC" {
            Some(Box::new(Self(nvic.clone())))
        } else {
            None
        }
    }
}

impl Peripheral for NvicWrapper {
    fn read(&mut self, sys: &System, offset: u32) -> u32 {
        self.0.borrow_mut().read(sys, offset)
    }

    fn write(&mut self, sys: &System, offset: u32, value: u32) {
        self.0.borrow_mut().write(sys, offset, value)
    }
}


/*
0xE000E100 B  REGISTER ISER0 (rw): Interrupt Set-Enable Register
0xE000E104 B  REGISTER ISER1 (rw): Interrupt Set-Enable Register
0xE000E108 B  REGISTER ISER2 (rw): Interrupt Set-Enable Register

0xE000E180 B  REGISTER ICER0 (rw): Interrupt Clear-Enable Register
0xE000E184 B  REGISTER ICER1 (rw): Interrupt Clear-Enable Register
0xE000E188 B  REGISTER ICER2 (rw): Interrupt Clear-Enable Register

0xE000E200 B  REGISTER ISPR0 (rw): Interrupt Set-Pending Register
0xE000E204 B  REGISTER ISPR1 (rw): Interrupt Set-Pending Register
0xE000E208 B  REGISTER ISPR2 (rw): Interrupt Set-Pending Register

0xE000E280 B  REGISTER ICPR0 (rw): Interrupt Clear-Pending Register
0xE000E284 B  REGISTER ICPR1 (rw): Interrupt Clear-Pending Register
0xE000E288 B  REGISTER ICPR2 (rw): Interrupt Clear-Pending Register

0xE000E300 B  REGISTER IABR0 (ro): Interrupt Active Bit Register
0xE000E304 B  REGISTER IABR1 (ro): Interrupt Active Bit Register
0xE000E308 B  REGISTER IABR2 (ro): Interrupt Active Bit Register

0xE000E400 B  REGISTER IPR0 (rw): Interrupt Priority Register
0xE000E404 B  REGISTER IPR1 (rw): Interrupt Priority Register
0xE000E408 B  REGISTER IPR2 (rw): Interrupt Priority Register
0xE000E40C B  REGISTER IPR3 (rw): Interrupt Priority Register
0xE000E410 B  REGISTER IPR4 (rw): Interrupt Priority Register
0xE000E414 B  REGISTER IPR5 (rw): Interrupt Priority Register
0xE000E418 B  REGISTER IPR6 (rw): Interrupt Priority Register
0xE000E41C B  REGISTER IPR7 (rw): Interrupt Priority Register
0xE000E420 B  REGISTER IPR8 (rw): Interrupt Priority Register
0xE000E424 B  REGISTER IPR9 (rw): Interrupt Priority Register
0xE000E428 B  REGISTER IPR10 (rw): Interrupt Priority Register
0xE000E42C B  REGISTER IPR11 (rw): Interrupt Priority Register
0xE000E430 B  REGISTER IPR12 (rw): Interrupt Priority Register
0xE000E434 B  REGISTER IPR13 (rw): Interrupt Priority Register
0xE000E438 B  REGISTER IPR14 (rw): Interrupt Priority Register
0xE000E43C B  REGISTER IPR15 (rw): Interrupt Priority Register
0xE000E440 B  REGISTER IPR16 (rw): Interrupt Priority Register
0xE000E444 B  REGISTER IPR17 (rw): Interrupt Priority Register
0xE000E448 B  REGISTER IPR18 (rw): Interrupt Priority Register
0xE000E44C B  REGISTER IPR19 (rw): Interrupt Priority Register
*/



/*
// SPDX-License-Identifier: GPL-3.0-or-later

use unicorn_engine::{Unicorn, RegisterARM};

#[derive(Default)]
struct Interrupts {
    // 128 different interrupts. Good enough for now
    pending: u128,
    in_interrupt: bool,
    vector_table_addr: u32,
}

impl Interrupts {
    pub fn new(vector_table_addr: u32) -> Self {
        Self { vector_table_addr, ..Default::default() }
    }

    pub fn invoke_pending(&mut self, uc: &mut Unicorn<()>) {
        if self.pending == 0 {
            return;
        }

        // Interrupt disabled?
        let primask = uc.reg_read(RegisterARM::PRIMASK).unwrap();
        if primask != 0 {
            return;
        }

        let intr = self.pending.trailing_zeros();
        let mut intr_addr = [0,0,0,0];
        uc.mem_read((self.vector_table_addr + intr * 4).into(), &mut intr_addr)
            .expect("Failed to read vector table");

        RegisterARM::S
        //let intr_addr = u32::from_le_bytes(intr_addr);

        //uc.reg_read(RegisterARM::


    }
}
*/
