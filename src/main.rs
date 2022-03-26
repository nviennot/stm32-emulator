// SPDX-License-Identifier: GPL-3.0-or-later

mod config;
mod emulator;
mod util;
mod peripherals;

use std::io::prelude::*;
use std::sync::atomic::Ordering::Relaxed;
use clap::Parser;
use clap::AppSettings;
use anyhow::{Result, Context};
use env_logger::fmt::WriteStyle;
use log::LevelFilter;

use config::Config;
use emulator::run_emulator;
use util::read_file_str;


#[macro_use]
extern crate log;

/// STM32 Emulator
#[derive(Parser, Debug)]
#[clap(
    global_setting(AppSettings::DeriveDisplayOrder)
)]
pub struct Args {
    /// Config file
    config: String,

    /// Verbosity. Can be repeated. -vvv is the maximum.
    #[clap(short, long, parse(from_occurrences))]
    verbose: u8,

    /// Maximum number of instructions to execute
    #[clap(short, long)]
    max_instructions: Option<u64>,

    /// Stop emulation when pc reaches this address
    #[clap(short, long, parse(try_from_str=clap_num::maybe_hex))]
    stop_addr: Option<u32>,

    /// Colorize output
    #[clap(short, long, arg_enum, default_value="auto")]
    color: Color,
}

#[derive(clap::ArgEnum, Clone, Copy, Debug)]
enum Color {
    Auto,
    Always,
    Never,
}

impl std::convert::From<Color> for WriteStyle {
    fn from(c: Color) -> Self {
        match c {
            Color::Always => WriteStyle::Always,
            Color::Never => WriteStyle::Never,
            Color::Auto => WriteStyle::Auto,
        }
    }
}


fn init_logging(args: &Args) {
    let lf = match args.verbose {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    static mut LAST_NUM_INSTRUCTIONS: u64 = 0;

    env_logger::Builder::new()
        .filter_level(lf)
        .write_style(args.color.into())
        .target(env_logger::Target::Stdout)
        .format(|buf, record| {
            use env_logger::fmt::Color;
            let num_instructions = emulator::NUM_INSTRUCTIONS.load(Relaxed);
            let delta_instructions = num_instructions - unsafe { LAST_NUM_INSTRUCTIONS };
            unsafe { LAST_NUM_INSTRUCTIONS = num_instructions };
            let pc = unsafe { emulator::LAST_INSTRUCTION.0 };

            let mut style = buf.style();
            let level = match record.level() {
                log::Level::Error => style.set_color(Color::Red).set_intense(true).value("ERROR"),
                log::Level::Warn =>  style.set_color(Color::Yellow).set_intense(true).value("WARN "),
                log::Level::Info =>  style.set_color(Color::Green).set_intense(true).value("INFO "),
                log::Level::Debug => style.set_color(Color::Cyan).set_intense(true).value("DEBUG"),
                log::Level::Trace => style.set_color(Color::Blue).set_intense(true).value("TRACE"),
            };

            let mut style = buf.style();
            style.set_color(Color::Black).set_intense(true);
            let header = format!("[tsc={:08} dtsc=+{:08} pc=0x{:08x}]", num_instructions, delta_instructions, pc);
            let header = style.value(header);

            writeln!(buf, "{} {} {}", header, level, record.args())
        })
        .init();
}

fn main() -> Result<()> {
    let args = Args::parse();
    init_logging(&args);

    let config: Config = serde_yaml::from_str(&read_file_str(&args.config)?)
        .with_context(|| format!("Failed to parse {}", args.config))?;

    let device = svd_parser::parse(&read_file_str(&config.cpu.svd)?)
        .with_context(|| format!("Failed to parse {}", config.cpu.svd))?;

    run_emulator(config, device, args)
}
