// SPDX-License-Identifier: GPL-3.0-or-later

use std::{rc::Rc, cell::RefCell};
use unicorn_engine::{Unicorn, unicorn_const::Permission};
use crate::{peripherals::Peripherals, ext_devices::ExtDevices, util::{UniErr, round_up, self}, config::Config, sdl::Sdl};
use anyhow::{Context as _, Result};
use svd_parser::svd::Device as SvdDevice;

// System is passed around during read/write hooks
pub struct System<'a, 'b> {
    // not sure how to not have a refcell here
    pub uc: RefCell<&'a mut Unicorn<'b, ()>>,
    pub p: Rc<Peripherals>,
    pub d: Rc<ExtDevices>,
}

fn bind(
    uc: &mut Unicorn<()>,
    peripherals: &Rc<Peripherals>,
    ext_devices: &Rc<ExtDevices>,
) -> Result<()> {
    for (start, end) in Peripherals::MEMORY_MAPS {
        let read_cb = {
            let peripherals = peripherals.clone();
            let ext_devices = ext_devices.clone();
            move |uc: &mut Unicorn<'_, ()>, addr, size| {
                let mut sys = System {
                    uc: RefCell::new(uc),
                    p: peripherals.clone(),
                    d: ext_devices.clone(),
                };
                peripherals.read(&mut sys, start + addr as u32, size as u8) as u64
            }
        };

        let write_cb = {
            let peripherals = peripherals.clone();
            let ext_devices = ext_devices.clone();
            move |uc: &mut Unicorn<'_, ()>, addr, size, value| {
                let mut sys = System {
                    uc: RefCell::new(uc),
                    p: peripherals.clone(),
                    d: ext_devices.clone(),
                };
                peripherals.write(&mut sys, start + addr as u32, size as u8, value as u32)
            }
        };

        uc.mmio_map(start as u64, (end-start) as usize, Some(read_cb), Some(write_cb))
            .map_err(UniErr).context("Failed to mmio_map()")?;
    }

    Ok(())
}

fn load_memory_regions(uc: &mut Unicorn<()>, config: &Config) -> Result<()> {
    for region in &config.regions {
        debug!("Mapping region start=0x{:08x} len=0x{:x} name={}",
            region.start, region.size, region.name);

        let size = round_up(region.size as usize, 4096);
        uc.mem_map(region.start.into(), size, Permission::ALL)
            .map_err(UniErr).with_context(||
                format!("Memory mapping of peripheral={} failed", region.name))?;

        if let Some(ref load) = region.load {
            info!("Loading file={} at base=0x{:08x}", load, region.start);
            let content = util::read_file(load)?;
            let content = &content[0..content.len().min(size)];
            uc.mem_write(region.start.into(), content).map_err(UniErr)?;
        }
    }

    for patch in config.patches.as_ref().unwrap_or(&vec![]) {
        uc.mem_write(patch.start.into(), &patch.data)
            .map_err(UniErr).with_context(||
                format!("Failed to apply patch at addr={}", patch.start))?;
    }

    Ok(())
}

pub fn prepare<'a, 'b>(uc: &'a mut Unicorn<'b, ()>, config: Config, svd_device: SvdDevice) -> Result<System<'a, 'b>> {
    load_memory_regions(uc, &config)?;

    let sdl = Rc::new(RefCell::new(Sdl::new()));
    let ext_devices = Rc::new(ExtDevices::new(config.devices.unwrap_or(Default::default()), &sdl)?);
    let peripherals = Rc::new(Peripherals::from_svd(svd_device, &ext_devices));

    bind(uc, &peripherals, &ext_devices)?;

    Ok(System {
        uc: RefCell::new(uc),
        p: peripherals,
        d: ext_devices,
    })
}
