// SPDX-License-Identifier: GPL-3.0-or-later

use unicorn_engine::Unicorn;

use super::{Peripheral, Peripherals};

#[derive(Default)]
pub struct Dma {
    name: String,
    streams: [Stream; 8],
}

impl Dma {
    pub fn new(name: &str) -> Option<Box<dyn Peripheral>> {
        if name.starts_with("DMA") {
            let name = name.to_string();
            Some(Box::new(Self { name, ..Self::default() }))
        } else {
            None
        }
    }
}

impl Peripheral for Dma {
    fn read(&mut self, _perifs: &Peripherals, uc: &mut Unicorn<()>, offset: u32) -> u32 {
        match Access::from_offset(offset) {
            Access::StreamReg(i, offset) => self.streams[i].read(&self.name, uc, offset),
            _ => 0
        }
    }

    fn write(&mut self, _perifs: &Peripherals, uc: &mut Unicorn<()>, offset: u32, value: u32) {
        match Access::from_offset(offset) {
            Access::StreamReg(i, offset) => self.streams[i].write(&self.name, uc, offset, value),
            _ => {}
        }
    }
}

#[derive(Default)]
struct Stream {
    pub cr: u32,
    pub next_cr: Option<u32>,
    pub ndtr: u32,
    pub par: u32,
    pub m0ar: u32,
    pub m1ar: u32,
    pub fcr: u32,
}

impl Stream {
    fn channel(&self) -> u8 {
        ((self.cr >> 25) & 0b111) as u8
    }

    fn dir(&self) -> Dir {
        match (self.cr >> 6) & 0b11 {
            0b00 => Dir::Read,
            0b01 => Dir::Write,
            0b10 => Dir::MemCopy,
            _ => Dir::Invalid,
        }
    }

    // 1, 2, 4 (8bit, 16bit, 32bit)
    fn word_size(&self) -> usize {
        match (self.cr >> 11) & 0b11 {
            0b00 => 1,
            0b01 => 2,
            0b10 => 4,
            _ => 1,
        }
    }

    fn data_size(&self) -> usize {
        self.word_size() * self.ndtr as usize
    }

    fn data_addr(&self) -> u32 {
        if (self.cr >> 19) & 1 != 0 {
            self.m1ar
        } else {
            self.m0ar
        }
    }

    pub fn read(&mut self, _name: &str, _uc: &mut Unicorn<()>, offset: u32) -> u32 {
        match offset {
            0x0000 => {
                let v = self.cr;
                if let Some(next_cr) = self.next_cr {
                    self.cr = next_cr;
                }
                v
            }
            0x0004 => self.ndtr,
            0x0008 => self.par,
            0x000c => self.m0ar,
            0x0010 => self.m1ar,
            0x0014 => self.fcr,
            _ => 0
        }
    }

    pub fn write(&mut self, name: &str, uc: &mut Unicorn<()>, offset: u32, mut value: u32)  {
        match offset {
            0x0000 => {
                self.cr = value;

                // CRx register
                if value & 1 != 0 {
                    // Enable!
                    let addr = self.data_addr();
                    let peri = self.par;
                    let size = self.data_size();
                    let buf = uc.mem_read_as_vec(addr.into(), size);
                    debug!("{} xfer initiated channel={} peri=0x{:08x} dir={:?} addr=0x{:08x} size={} buf={:x?}",
                        name, self.channel(), peri, self.dir(), addr, size, buf);
                    value &= !1;
                    self.ndtr = 0;
                    self.next_cr = Some(value);
                }
            }
            0x0004 => { self.ndtr = value; }
            0x0008 => { self.par = value; }
            0x000c => { self.m0ar = value; }
            0x0010 => { self.m1ar = value; }
            0x0014 => { self.fcr = value; }
            _ => {}
        }
    }
}

#[derive(Debug)]
enum Dir {
    Read,
    Write,
    MemCopy,
    Invalid,
}

enum Access {
    Reg(u32),
    /// CR0, CR1, etc.
    StreamReg(usize, u32),
}

impl Access {
    pub fn from_offset(offset: u32) -> Self {
        if offset < 0x28 {
            Access::Reg(offset)
        } else {
            let stride = 0x18;
            let start = 0x10;

            let offset = offset - start;
            Access::StreamReg(
                (offset / stride) as usize,
                offset % stride
            )
        }
    }
}
