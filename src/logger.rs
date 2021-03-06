use std::fs;
use std::io;
use std::io::prelude::*;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use log::Log;
use once_cell::sync::Lazy;

use crate::cache;

const LOG_FILENAME: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "-",
    env!("CARGO_PKG_VERSION"),
    ".log"
);
static LOGGER: Lazy<Logger> = Lazy::new(|| Logger::new().unwrap());

struct Logger {
    file: Arc<Mutex<fs::File>>,
}

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let time = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S");
            let mut f = self.file.lock().unwrap();
            writeln!(f, "[{}] [{}] {}", time, record.level(), record.args()).unwrap();
        }
    }

    fn flush(&self) {
        let mut f = self.file.lock().unwrap();
        f.flush().unwrap();
    }
}

impl Logger {
    fn new() -> io::Result<Self> {
        fs::create_dir_all(&*cache::DIR)?;
        let path = cache::DIR.join(LOG_FILENAME);
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        let file = Arc::new(Mutex::new(file));
        Ok(Self { file })
    }
}

pub fn init() -> Result<()> {
    log::set_logger(&*LOGGER)?;
    log::set_max_level(log::LevelFilter::Info);
    Ok(())
}
