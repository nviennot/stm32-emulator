// SPDX-License-Identifier: GPL-3.0-or-later

use std::error::Error;
use std::io::prelude::*;
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
