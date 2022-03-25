use std::{collections::HashMap, mem::MaybeUninit, cell::RefCell, rc::Rc};
use svd_parser::svd::Device;
use unicorn_engine::{unicorn_const::{Arch, Mode, Permission, HookType}, Unicorn, RegisterARM};
use crate::{config::Config, util::{round_up, UniErr, read_file}, peripherals::Peripherals};
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

    Ok(())
}

pub fn init_peripherals(uc: &mut Unicorn<()>, mut device: Device) -> Result<Rc<RefCell<Peripherals>>> {
    device.peripherals.sort_by_key(|f| f.base_address);

    let peripherals = device.peripherals.iter()
        .map(|d| (d.name.to_string(), d))
        .collect::<HashMap<_,_>>();

    for p in &device.peripherals {
        let name = &p.name;
        let base = p.base_address;

        let p = if let Some(derived_from) = p.derived_from.as_ref() {
            peripherals.get(derived_from)
                .as_ref()
                .unwrap_or_else(|| panic!("Cannot find peripheral {}", derived_from))
        } else {
            p
        };

        for reg in p.all_registers() {
            debug!("Peripheral base=0x{:x} name={:?} reg={:?} offset={:x?}", base, name, reg.display_name, reg.address_offset);
        }
    }

    let peripherals = Peripherals::new();

    let peripherals = Rc::new(RefCell::new(peripherals));

    for (start, end) in Peripherals::MEMORY_MAPS {
        let read_peripherals = peripherals.clone();
        let write_peripherals = peripherals.clone();
        let read_cb =  move |uc: &mut Unicorn<'_, ()>, addr, size| {
            read_peripherals.borrow_mut().read(uc, start + addr as u32) as u64
        };
        let write_cb = move |uc: &mut Unicorn<'_, ()>, addr, size, value| {
            write_peripherals.borrow_mut().write(uc, start + addr as u32, value as u32)
        };

        uc.mmio_map(start as u64, (end-start) as usize, Some(read_cb), Some(write_cb))
            .map_err(UniErr).context("Failed to mmio_map()")?;
    }

    Ok(peripherals)
}

pub fn run_emulator(config: Config, device: Device) -> Result<()> {
    let mut uc = Unicorn::new(Arch::ARM, Mode::LITTLE_ENDIAN)
        .map_err(UniErr).context("Failed to initialize Unicorn instance")?;

    load_memory_regions(&mut uc, &config)?;
    init_peripherals(&mut uc, device)?;

    uc.add_mem_hook(HookType::MEM_UNMAPPED, 0, u64::MAX, |emu, type_, addr, size, value| {
        let pc = emu.reg_read(RegisterARM::PC).expect("failed to get pc");
        error!("mem: {:?} inst_addr=0x{:08x} mem_addr=0x{:08x}, size={} value={}", type_, pc, addr, size, value);
        false
    }).expect("add_mem_hook failed");

    uc.add_code_hook(0, u64::MAX, |_emu, _addr, _size| {
    }).expect("add_code_hook failed");

    let vector_table = VectorTable::from_memory(&uc, config.cpu.vector_table)?;
    uc.reg_write(RegisterARM::SP, vector_table.sp.into()).map_err(UniErr)?;

    info!("Starting emulation");
    uc.emu_start(vector_table.reset.into(), 0, 0, 0).map_err(UniErr)?;

    info!("Done");


    Ok(())
}
