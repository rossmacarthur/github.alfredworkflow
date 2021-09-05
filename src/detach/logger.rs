use std::fs;
use std::io::prelude::*;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use log::Log;
use powerpack::env;

const LOG_FILENAME: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "-",
    env!("CARGO_PKG_VERSION"),
    ".log"
);

pub struct Logger {
    file: Arc<Mutex<fs::File>>,
}

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        true
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
    pub fn new() -> Result<Self> {
        let cache_dir = env::workflow_cache().context("failed to find cache directory")?;
        fs::create_dir_all(&cache_dir)?;
        let path = cache_dir.join(LOG_FILENAME);
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        let file = Arc::new(Mutex::new(file));
        Ok(Self { file })
    }
}
