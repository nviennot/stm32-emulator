// SPDX-License-Identifier: GPL-3.0-or-later

use std::error::Error;
use std::io::prelude::*;
use svd_parser::svd::{MaybeArray, RegisterInfo, PeripheralInfo};
use unicorn_engine::unicorn_const::uc_error;
use anyhow::{Context, Result};

#[derive(Debug)]
pub struct UniErr(pub uc_error);
impl core::fmt::Display for UniErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unicorn error={:?}", self.0)
    }
}
impl Error for UniErr {}

pub fn round_up(n: usize, boundary: usize) -> usize {
    ((n + boundary - 1) / boundary) * boundary
}

pub fn read_file(path: &str) -> Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open {}", path))?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .with_context(|| format!("Failed to read {}", path))?;

    Ok(buf)
}


pub fn read_file_str(path: &str) -> Result<String> {
    let content = read_file(path)?;
    let str = String::from_utf8(content)?;
    Ok(str)
}


pub fn extract_svd_registers(p: &MaybeArray<PeripheralInfo>) -> Vec<RegisterInfo> {
    fn collect_register(reg: &RegisterInfo, in_array: Option<(u32, String)>, cluster: Option<(u32, &str)>) -> RegisterInfo {
        let mut reg = reg.clone();

        if let Some((array_address, name)) = in_array {
            reg.address_offset = array_address;
            reg.name = name;
        }

        if let Some((cluster_offset, cluster_suffix)) = cluster {
            reg.address_offset += cluster_offset;
            reg.name.push_str(cluster_suffix);
        }
        reg
    }

    fn collect_registers<'a>(regs: impl IntoIterator<Item=&'a MaybeArray<RegisterInfo>>, cluster: Option<(u32, &str)>) -> Vec<RegisterInfo> {
        regs.into_iter().flat_map(|r| {
            match r {
                MaybeArray::Single(r) => {
                    vec![collect_register(r, None, cluster)].into_iter()
                }
                MaybeArray::Array(r, dim) => {
                    let offsets = svd_parser::svd::register::address_offsets(&r, &dim);
                    let names = svd_parser::svd::array::names(r, dim);
                    offsets.zip(names)
                        .map(|in_array| collect_register(r, Some(in_array), cluster))
                        .collect::<Vec<_>>()
                        .into_iter()
                }
            }
        })
        .collect()
    }

    let mut all_regs = collect_registers(p.registers(), None);

    for cluster in p.clusters() {
        match cluster {
            MaybeArray::Single(c) => {
                all_regs.append(&mut collect_registers(c.all_registers(), None));
            }
            MaybeArray::Array(c, dim) => {
                let offsets = svd_parser::svd::cluster::address_offsets(c, dim);
                let indexes = dim.indexes();


                for (offset, index) in offsets.zip(indexes) {
                    all_regs.append(&mut collect_registers(c.all_registers(), Some((offset, &index))));
                }
            }
        }
    }

    all_regs
}


#[derive(Default, Debug)]
pub struct Point {
    pub x: u16,
    pub y: u16,
}

#[derive(Default, Debug)]
pub struct Rect {
    pub left: u16,
    pub right: u16,
    pub top: u16,
    pub bottom: u16,
}
