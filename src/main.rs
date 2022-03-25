mod config;
mod emulator;
mod util;
mod peripherals;

use std::io::prelude::*;
use std::time::Instant;

use clap::Parser;
use clap::AppSettings;
use anyhow::{Result, Context};
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
struct Args {
    /// Config file
    config: String,

    // Verbosity. Can be repeated
    #[clap(short, long, parse(from_occurrences))]
    verbose: u8
}

fn init_logging(level: u8) {
    let lf = match level {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    lazy_static::lazy_static! {
         static ref START_TIME: Instant = Instant::now();
    }

    lazy_static::initialize(&START_TIME);

    env_logger::Builder::new()
        .filter_level(lf)
        .format(|buf, record| {
            let time = Instant::now().duration_since(*START_TIME);
            let time = (time.as_millis() as f32)/1000.0;
            writeln!(buf, "[{:02.3}s] {}", time, record.args())
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

    run_emulator(config, device)
}
