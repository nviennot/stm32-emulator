// SPDX-License-Identifier: GPL-3.0-or-later

use std::{rc::Rc, cell::RefCell};
use unicorn_engine::{Unicorn, unicorn_const::Permission};
use crate::{peripherals::{Peripherals, gpio::GpioPorts}, ext_devices::ExtDevices, util::{UniErr, round_up, self}, config::Config, framebuffers::Framebuffers};
use anyhow::{Context as _, Result};
use svd_parser::svd::Device as SvdDevice;

// System is passed around during read/write hooks. It's more convenient than passing each thing individually.
// Maybe it should just be a global variable, and we call it a day.
pub struct System<'a, 'b> {
    // not sure how to not have a refcell here
    pub uc: RefCell<&'a mut Unicorn<'b, ()>>,
    // Sorry for the single letter variables, it just gets too verbose at times.
    pub p: Rc<Peripherals>,
    pub d: Rc<ExtDevices>,
}

impl<'a, 'b> System<'a, 'b> {
    fn new(uc: &'a mut Unicorn<'b, ()>, p: Peripherals,  d: ExtDevices) -> Self {
        Self {
            uc: RefCell::new(uc),
            p: Rc::new(p),
            d: Rc::new(d),
        }
    }

    fn bind_peripherals_to_unicorn(&mut self) -> Result<()> {
        for (start, end) in Peripherals::MEMORY_MAPS {
            let read_cb = {
                let p = self.p.clone();
                let d = self.d.clone();
                move |uc: &mut Unicorn<'_, ()>, addr, size| {
                    let mut sys = System { uc: RefCell::new(uc), p: p.clone(), d: d.clone() };
                    p.read(&mut sys, start + addr as u32, size as u8) as u64
                }
            };

            let write_cb = {
                let p = self.p.clone();
                let d = self.d.clone();
                move |uc: &mut Unicorn<'_, ()>, addr, size, value| {
                    let mut sys = System { uc: RefCell::new(uc), p: p.clone(), d: d.clone() };
                    p.write(&mut sys, start + addr as u32, size as u8, value as u32)
                }
            };

            self.uc.borrow_mut().mmio_map(start as u64, (end-start) as usize, Some(read_cb), Some(write_cb))
                .map_err(UniErr).context("Failed to mmio_map()")?;
        }

        Ok(())
    }
}

fn load_memory_regions(uc: &mut Unicorn<()>, config: &Config) -> Result<()> {
    for region in &config.regions {
        debug!("Mapping region start=0x{:08x} len=0x{:x} name={}",
            region.start, region.size, region.name);

        let size = round_up(region.size as usize, 4096); // magic number is from mem_map() documentation
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

pub fn prepare<'a, 'b>(uc: &'a mut Unicorn<'b, ()>, config: Config, svd_device: SvdDevice)
-> Result<(System<'a, 'b>, Framebuffers)>
  {
    load_memory_regions(uc, &config)?;

    let framebuffers = Framebuffers::from_config(config.framebuffers.unwrap_or_default());
    let mut gpio: GpioPorts = Default::default();
    let ext_devices = config.devices.unwrap_or_default().into_ext_devices(&mut gpio, &framebuffers)?;
    let peripherals = Peripherals::from_svd(svd_device, config.peripherals.unwrap_or_default(), gpio, &ext_devices);

    let mut system = System::new(uc, peripherals, ext_devices);
    system.bind_peripherals_to_unicorn()?;
    Ok((system, framebuffers))
}
