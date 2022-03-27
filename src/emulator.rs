// SPDX-License-Identifier: GPL-3.0-or-later

use std::{mem::MaybeUninit, sync::atomic::{AtomicU64, Ordering, AtomicBool}};
use svd_parser::svd::Device as SvdDevice;
use unicorn_engine::{unicorn_const::{Arch, Mode, HookType, MemType}, Unicorn, RegisterARM};
use crate::{config::Config, util::UniErr, Args};
use anyhow::{Context as _, Result};

#[repr(C)]
struct VectorTable {
    pub sp: u32,
    pub reset: u32,
}

impl VectorTable {
    pub fn from_memory(uc: &Unicorn<()>, addr: u32) -> Result<Self> {
        unsafe {
            let mut self_ = MaybeUninit::<Self>::uninit();
            let buf = std::slice::from_raw_parts_mut(self_.as_mut_ptr() as *mut u8, std::mem::size_of::<Self>());
            uc.mem_read(addr.into(), buf).map_err(UniErr)?;
            Ok(self_.assume_init())
        }
    }
}

fn thumb(pc: u64) -> u64 {
    pc | 1
}

// PC + instruction size
pub static mut LAST_INSTRUCTION: (u32, u8) = (0,0);
pub static NUM_INSTRUCTIONS: AtomicU64 = AtomicU64::new(0);
static CONTINUE_EXECUTION: AtomicBool = AtomicBool::new(false);

pub fn run_emulator(config: Config, svd_device: SvdDevice, args: Args) -> Result<()> {
    let mut uc = Unicorn::new(Arch::ARM, Mode::MCLASS | Mode::LITTLE_ENDIAN)
        .map_err(UniErr).context("Failed to initialize Unicorn instance")?;

    let vector_table_addr = config.cpu.vector_table;

    let sys = crate::system::prepare(&mut uc, config, svd_device)?;
    let display = sys.d.displays.first().map(|d| d.clone());

    // Important to keep. Otherwise pc is not accurate due to prefetching and all.
    let trace_instructions = crate::verbose() >= 4;
    let busy_loop_stop = args.busy_loop_stop;
    uc.add_code_hook(0, u64::MAX, move |uc, addr, size| {
        unsafe {
            if busy_loop_stop && LAST_INSTRUCTION.0 == addr as u32 {
                info!("Busy loop reached");
                uc.emu_stop().unwrap();
            }
            LAST_INSTRUCTION = (addr as u32, size as u8);
        }
        NUM_INSTRUCTIONS.fetch_add(1, Ordering::Acquire);
        if trace_instructions {
            trace!("step");
        }
    }).expect("add_code_hook failed");

    uc.add_intr_hook(|_uc, addr| {
        trace!("intr_hook {:08x}", addr);
    }).expect("add_intr_hook failed");

    uc.add_mem_hook(HookType::MEM_UNMAPPED, 0, u64::MAX, |uc, type_, addr, size, value| {
        if type_ == MemType::WRITE_UNMAPPED {
            warn!("{:?} addr=0x{:08x} size={} value=0x{:08x}", type_, addr, size, value);
        } else {
            warn!("{:?} addr=0x{:08x} size={}", type_, addr, size);
        }

        unsafe {
            let pc = uc.reg_read(RegisterARM::PC).expect("failed to get pc");
            assert!(pc as u32 == LAST_INSTRUCTION.0);
            uc.reg_write(RegisterARM::PC, thumb(pc as u64 + LAST_INSTRUCTION.1 as u64)).unwrap();
        }

        CONTINUE_EXECUTION.store(true, Ordering::Release);

        false
    }).expect("add_mem_hook failed");

    let vector_table = VectorTable::from_memory(&uc, vector_table_addr)?;
    uc.reg_write(RegisterARM::SP, vector_table.sp.into()).map_err(UniErr)?;

    info!("Starting emulation");

    let mut pc = vector_table.reset as u64;

    loop {
        let max_instructions = args.max_instructions.map(|c|
            c - NUM_INSTRUCTIONS.load(Ordering::Relaxed)
        );
        if max_instructions == Some(0) {
            info!("Reached target address. Done");
            break;
        }

        let result = uc.emu_start(
            pc,
            args.stop_addr.unwrap_or(0) as u64,
            0,
            max_instructions.unwrap_or(0) as usize,
        ).map_err(UniErr);
        pc = uc.reg_read(RegisterARM::PC).expect("failed to get pc");

        if CONTINUE_EXECUTION.swap(false, Ordering::AcqRel) {
            if crate::verbose() >= 3 {
                trace!("Resuming execution pc={:08x}", pc);
            }
            pc = thumb(pc);
            continue;
        }

        info!("Emulation stop");
        result?;
        break;
    }

    if let Some(display) = display {
        display.borrow().write_framebuffer_to_file("framebuffer.bin")?;
    }


    Ok(())
}


    /*
    uc.add_insn_invalid_hook(|uc| {
        let pc = uc.reg_read(RegisterARM::PC).unwrap();
        let mut ins = [0,0,0,0];
        uc.mem_read(pc, &mut ins).unwrap();
        match ins {
            /*
            [0xef, 0xf3, 0x10, 0x80] => {
                // mrs r0, primask
                uc.reg_write(RegisterARM::R0, 0).unwrap();
                uc.reg_write(RegisterARM::PC, thumb(pc+4)).unwrap();
                trace!("read primask");
                true
            }
            [0x80, 0xf3, 0x10, 0x88] => {
                // msr primask,r0
                trace!("write primask");
                uc.reg_write(RegisterARM::PC, thumb(pc+4)).unwrap();
                true
            }
            [0x72, 0xb6, _, _] => {
                // instruction: cpsid
                trace!("Disabling interrupt");
                uc.reg_write(RegisterARM::PC, thumb(pc+2)).unwrap();
                trace!("disabled interrupts, pc is now {:08x}", uc.pc_read().unwrap());
                true
            }
            */
            _ => {
                error!("invalid insn: pc=0x{:08x}, ins={:x?}", pc, ins);
                false
            }
        }
    }).expect("add_insn_invalid_hook failed");
    */
