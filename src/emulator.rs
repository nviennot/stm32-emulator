// SPDX-License-Identifier: GPL-3.0-or-later

use std::{collections::HashMap, mem::MaybeUninit, cell::RefCell, rc::Rc, sync::atomic::{AtomicU64, Ordering, AtomicBool}};
use svd_parser::svd::Device;
use unicorn_engine::{unicorn_const::{Arch, Mode, Permission, HookType}, Unicorn, RegisterARM};
use crate::{config::Config, util::{round_up, UniErr, read_file}, peripherals::Peripherals, Args, devices::Devices};
use anyhow::{Context, Result};

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

pub fn load_memory_regions(uc: &mut Unicorn<()>, config: &Config) -> Result<()> {
    for region in &config.regions {
        debug!("Mapping region start=0x{:08x} len=0x{:x} name={}",
            region.start, region.size, region.name);

        let size = round_up(region.size as usize, 4096);
        uc.mem_map(region.start.into(), size, Permission::ALL)
            .map_err(UniErr).with_context(||
                format!("Memory mapping of peripheral={} failed", region.name))?;

        if let Some(ref load) = region.load {
            info!("Loading file={} at base=0x{:08x}", load, region.start);
            let content = read_file(load)?;
            uc.mem_write(region.start.into(), &content).map_err(UniErr)?;
        }
    }

    for patch in config.patches.as_ref().unwrap_or(&vec![]) {
        uc.mem_write(patch.start.into(), &patch.data)
            .map_err(UniErr).with_context(||
                format!("Failed to apply patch at addr={}", patch.start))?;
    }

    Ok(())
}

pub fn init_peripherals(uc: &mut Unicorn<()>, mut svd_device: Device, devices: &mut Devices) -> Result<Rc<RefCell<Peripherals>>> {
    let mut peripherals = Peripherals::new();

    svd_device.peripherals.sort_by_key(|f| f.base_address);
    let svd_peripherals = svd_device.peripherals.iter()
        .map(|d| (d.name.to_string(), d))
        .collect::<HashMap<_,_>>();

    for p in &svd_device.peripherals {
        let name = &p.name;
        let base = p.base_address;

        let p = if let Some(derived_from) = p.derived_from.as_ref() {
            svd_peripherals.get(derived_from)
                .as_ref()
                .unwrap_or_else(|| panic!("Cannot find peripheral {}", derived_from))
        } else {
            p
        };

        let regs: Vec<_> = p.all_registers().cloned().collect();
        peripherals.register_peripheral(name.to_string(), base as u32, &regs, devices);
    }


    let peripherals = Rc::new(RefCell::new(peripherals));

    for (start, end) in Peripherals::MEMORY_MAPS {
        let read_peripherals = peripherals.clone();
        let write_peripherals = peripherals.clone();
        let read_cb =  move |uc: &mut Unicorn<'_, ()>, addr, size| {
            read_peripherals.borrow_mut().read(uc, start + addr as u32, size as u8) as u64
        };
        let write_cb = move |uc: &mut Unicorn<'_, ()>, addr, size, value| {
            write_peripherals.borrow_mut().write(uc, start + addr as u32, size as u8, value as u32)
        };

        uc.mmio_map(start as u64, (end-start) as usize, Some(read_cb), Some(write_cb))
            .map_err(UniErr).context("Failed to mmio_map()")?;
    }

    Ok(peripherals)
}

fn thumb(pc: u64) -> u64 {
    pc | 1
}

// PC + instruction size
pub static mut LAST_INSTRUCTION: (u32, u8) = (0,0);
pub static NUM_INSTRUCTIONS: AtomicU64 = AtomicU64::new(0);
static CONTINUE_EXECUTION: AtomicBool = AtomicBool::new(false);

pub fn run_emulator(config: Config, device: Device, args: Args) -> Result<()> {
    let mut uc = Unicorn::new(Arch::ARM, Mode::MCLASS | Mode::LITTLE_ENDIAN)
        .map_err(UniErr).context("Failed to initialize Unicorn instance")?;

    load_memory_regions(&mut uc, &config)?;

    let mut devices = config.devices.unwrap_or(Default::default()).try_into()?;
    init_peripherals(&mut uc, device, &mut devices)?;
    devices.assert_empty()?;

    // Important to keep. Otherwise pc is not accurate due to prefetching and all.
    let trace_instructions = args.verbose >= 3;
    uc.add_code_hook(0, u64::MAX, move |_uc, addr, size| {
        if trace_instructions {
            trace!("pc=0x{:08x}", addr);
        }
        unsafe { LAST_INSTRUCTION = (addr as u32, size as u8) };
        NUM_INSTRUCTIONS.fetch_add(1, Ordering::Acquire);
    }).expect("add_code_hook failed");

    uc.add_intr_hook(|_uc, addr| {
        trace!("intr_hook {:08x}", addr);
    }).expect("add_intr_hook failed");

    uc.add_mem_hook(HookType::MEM_UNMAPPED, 0, u64::MAX, |uc, type_, addr, size, value| {
        let pc = uc.reg_read(RegisterARM::PC).expect("failed to get pc");
        error!("{:?} pc=0x{:08x} addr=0x{:08x} size={} value=0x{:08x}", type_, pc, addr, size, value);

        unsafe {
            assert!(pc as u32 == LAST_INSTRUCTION.0);
            uc.reg_write(RegisterARM::PC, thumb(pc as u64 + LAST_INSTRUCTION.1 as u64)).unwrap();
        }

        CONTINUE_EXECUTION.store(true, Ordering::Release);

        false
    }).expect("add_mem_hook failed");

    let vector_table = VectorTable::from_memory(&uc, config.cpu.vector_table)?;
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
            trace!("Resuming execution pc={:08x}", pc);
            pc = thumb(pc);
            continue;
        }

        info!("Execution done. pc=0x{:08x}", pc);
        result?;
        break;
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
