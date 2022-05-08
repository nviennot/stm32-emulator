// SPDX-License-Identifier: GPL-3.0-or-later

use std::{mem::MaybeUninit, sync::atomic::{AtomicU64, Ordering, AtomicBool}, cell::RefCell};
use svd_parser::svd::Device as SvdDevice;
use unicorn_engine::{unicorn_const::{Arch, Mode, HookType, MemType}, Unicorn, RegisterARM};
use crate::{config::Config, util::UniErr, Args, system::System};
use anyhow::{Context as _, Result, bail};

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
static BUSY_LOOP_REACHED: AtomicBool = AtomicBool::new(false);

pub fn run_emulator(config: Config, svd_device: SvdDevice, args: Args) -> Result<()> {
    let mut uc = Unicorn::new(Arch::ARM, Mode::MCLASS | Mode::LITTLE_ENDIAN)
        .map_err(UniErr).context("Failed to initialize Unicorn instance")?;

    let vector_table_addr = config.cpu.vector_table;

    let sys = crate::system::prepare(&mut uc, config, svd_device)?;
    let display = sys.d.displays.first().map(|d| d.clone());

    // We hook on each instructions, but we could skip this.
    // The slowdown is less than 50%. It's okay for now.
    {
        let trace_instructions = crate::verbose() >= 4;
        let busy_loop_stop = args.busy_loop_stop;
        let p = sys.p.clone();
        let d = sys.d.clone();
        let interrupt_period = args.interrupt_period;
        sys.uc.borrow_mut().add_code_hook(0, u64::MAX, move |uc, addr, size| {
            unsafe {
                if busy_loop_stop && LAST_INSTRUCTION.0 == addr as u32 {
                    info!("Busy loop reached");
                    uc.emu_stop().unwrap();
                    BUSY_LOOP_REACHED.store(true, Ordering::Release);
                }
                LAST_INSTRUCTION = (addr as u32, size as u8);
            }

            let n = NUM_INSTRUCTIONS.fetch_add(1, Ordering::Acquire);
            if trace_instructions {
                trace!("step");
            }

            if n % interrupt_period as u64 == 0 {
                let sys = System { uc: RefCell::new(uc), p: p.clone(), d: d.clone() };
                p.nvic.borrow_mut().run_pending_interrupts(&sys, vector_table_addr);
            }
        }).expect("add_code_hook failed");
    }

    {
        let p = sys.p.clone();
        let d = sys.d.clone();
        sys.uc.borrow_mut().add_intr_hook(move |uc, exception| {
            match exception {
                /*
                    EXCP_UDEF            1   /* undefined instruction */
                    EXCP_SWI             2   /* software interrupt */
                    EXCP_PREFETCH_ABORT  3
                    EXCP_DATA_ABORT      4
                    EXCP_IRQ             5
                    EXCP_FIQ             6
                    EXCP_BKPT            7
                    EXCP_EXCEPTION_EXIT  8   /* Return from v7M exception.  */
                    EXCP_KERNEL_TRAP     9   /* Jumped to kernel code page.  */
                    EXCP_HVC            11   /* HyperVisor Call */
                    EXCP_HYP_TRAP       12
                    EXCP_SMC            13   /* Secure Monitor Call */
                    EXCP_VIRQ           14
                    EXCP_VFIQ           15
                    EXCP_SEMIHOST       16   /* semihosting call */
                    EXCP_NOCP           17   /* v7M NOCP UsageFault */
                    EXCP_INVSTATE       18   /* v7M INVSTATE UsageFault */
                    EXCP_STKOF          19   /* v8M STKOF UsageFault */
                    EXCP_LAZYFP         20   /* v7M fault during lazy FP stacking */
                    EXCP_LSERR          21   /* v8M LSERR SecureFault */
                    EXCP_UNALIGNED      22   /* v7M UNALIGNED UsageFault */
                    */
                8 => {
                    // Return from interrupt
                    let sys = System { uc: RefCell::new(uc), p: p.clone(), d: d.clone() };
                    p.nvic.borrow_mut().return_from_interrupt(&sys);
                    p.nvic.borrow_mut().run_pending_interrupts(&sys, vector_table_addr);
                }
                _ => {
                    error!("intr_hook intno={:08x}", exception);
                    std::process::exit(1);
                }
            }
        }).expect("add_intr_hook failed");
    }

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
            // yes, we want to panic if this goes negative.
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

        if let Err(e) = result {
            if CONTINUE_EXECUTION.swap(false, Ordering::AcqRel) {
                // This was a bad memory access, we keep going.
                if crate::verbose() >= 3 {
                    trace!("Resuming execution pc={:08x}", pc);
                }
                pc = thumb(pc);
                continue;
            } else {
                bail!(e);
            }
        }

        if args.stop_addr == Some(pc as u32) {
            info!("Stop address reached, stopping");
            break;
        }

        if BUSY_LOOP_REACHED.load(Ordering::Relaxed) {
            break;
        }
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
