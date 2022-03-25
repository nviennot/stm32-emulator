mod config;
mod emulator;
mod util;
mod peripherals;

use std::io::prelude::*;
use std::sync::atomic::Ordering::Relaxed;
use clap::Parser;
use clap::AppSettings;
use anyhow::{Result, Context};
use env_logger::fmt::Color;
use log::LevelFilter;

use config::Config;
use emulator::run_emulator;
use util::read_file_str;


#[macro_use]
extern crate log;

/// Decompress .data section from a Keil compiled firmware
#[derive(Parser, Debug)]
#[clap(
    global_setting(AppSettings::DeriveDisplayOrder)
)]
pub struct Args {
    /// Config file
    config: String,

    /// Verbosity. Can be repeated
    #[clap(short, long, parse(from_occurrences))]
    verbose: u8,

    /// Max instruction counts
    #[clap(short, long)]
    max_instructions: Option<u64>,
}


fn init_logging(level: u8) {
    let lf = match level {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    static mut LAST_NUM_INSTRUCTIONS: u64 = 0;

    env_logger::Builder::new()
        .filter_level(lf)
        .target(env_logger::Target::Stdout)
        .format(|buf, record| {
            let num_instructions = emulator::NUM_INSTRUCTIONS.load(Relaxed);
            let delta_instructions = num_instructions - unsafe { LAST_NUM_INSTRUCTIONS };
            unsafe { LAST_NUM_INSTRUCTIONS = num_instructions };

            let mut style = buf.style();
            let level = match record.level() {
                log::Level::Error => style.set_color(Color::Red).set_intense(true).value("ERROR"),
                log::Level::Warn =>  style.set_color(Color::Yellow).set_intense(true).value("WARN "),
                log::Level::Info =>  style.set_color(Color::Green).set_intense(true).value("INFO "),
                log::Level::Debug => style.set_color(Color::Cyan).set_intense(true).value("DEBUG"),
                log::Level::Trace => style.set_color(Color::Blue).set_intense(true).value("TRACE"),
            };

            let mut style = buf.style();
            match delta_instructions {
                0..=299    => { }
                300..=2999 => { style.set_color(Color::Yellow); }
                3000..     => { style.set_color(Color::Magenta); }
            }
            let delta_instructions = style.value(delta_instructions);

            writeln!(buf, "[{:08} +{:08}] {} {}", num_instructions, delta_instructions, level, record.args())
        })
        .init();
}

fn main() -> Result<()> {
    let args = Args::parse();
    init_logging(args.verbose);

    let config: Config = serde_yaml::from_str(&read_file_str(&args.config)?)
        .with_context(|| format!("Failed to parse {}", args.config))?;

    let device = svd_parser::parse(&read_file_str(&config.cpu.svd)?)
        .with_context(|| format!("Failed to parse {}", config.cpu.svd))?;

    run_emulator(config, device, args)
}
