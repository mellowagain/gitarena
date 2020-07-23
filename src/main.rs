#![forbid(unsafe_code)]

use anyhow::{Result, Context};
use chrono::Local;
use fern::{Dispatch, log_file};
use log::{LevelFilter, info, warn, error, debug};
use std::env;
use std::fs;
use std::io::stdout;
use std::path::Path;

#[actix_rt::main]
async fn main() -> Result<()> {
    init_logger()?;

    Ok(())
}

fn init_logger() -> Result<()> {
    let logs_dir = Path::new("logs");

    if !logs_dir.is_dir() {
        // Check if `logs` is a file and not a directory
        if logs_dir.exists() {
            fs::remove_file(logs_dir).context("Unable to delete `logs` file.")?;
        }

        fs::create_dir(logs_dir).context("Unable to create `logs` directory.")?;
    }

    Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] {} {} - {}",
                record.target(),
                record.level(),
                record.file().unwrap_or("null"),
                message
            ))
        })
        .level(LevelFilter::Debug)
        .chain(stdout())
        .chain(log_file(format!("logs/{}.log", Local::now().timestamp_millis()))?)
        .apply()
        .context("Failed to initialize logger.")
}
