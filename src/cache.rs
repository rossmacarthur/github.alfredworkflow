use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use powerpack::detach;
use powerpack::env;
use serde::{Deserialize, Serialize};
use serde_json as json;

use crate::logger;

const UPDATE_INTERVAL: Duration = Duration::from_secs(60);

pub static DIR: Lazy<PathBuf> = Lazy::new(|| {
    env::workflow_cache().unwrap_or_else(|| {
        let bundle_id =
            env::workflow_bundle_id().unwrap_or_else(|| "io.macarthur.ross.github".into());
        home::home_dir()
            .unwrap()
            .join("Library/Caches/com.runningwithcrayons.Alfred/Workflow Data")
            .join(&*bundle_id)
    })
});

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Cache {
    checksum: [u8; 20],
    modified: SystemTime,
    data: json::Value,
}

pub fn load<F>(key: &str, checksum: [u8; 20], f: F) -> Result<json::Value>
where
    F: FnOnce() -> Result<json::Value>,
{
    let dir = DIR.join(key);
    let path = dir.join("data.json");

    let update_cache = || match update(&dir, &path, checksum, f) {
        Ok(true) => log::info!("fetched {} and updated cache", path.display()),
        Ok(false) => log::info!("another process updated cache for {}", path.display()),
        Err(err) => log::error!("{:#}", err),
    };

    match fs::read(&path) {
        Ok(data) => {
            let curr: Cache = json::from_slice(&data)?;
            let needs_update = curr.checksum != checksum || {
                let now = SystemTime::now();
                now.duration_since(curr.modified)? > UPDATE_INTERVAL
            };

            if needs_update {
                detach::spawn(update_cache)?;
            }

            Ok(curr.data)
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            fs::create_dir_all(&dir)?;

            detach::spawn(update_cache)?;

            // wait up to 5 seconds for the cache to be populated
            let start = Instant::now();
            let poll_duration = Duration::from_secs(5);
            while Instant::now().duration_since(start) < poll_duration {
                thread::sleep(Duration::from_millis(200));
                if let Ok(data) = fs::read(&path) {
                    let curr: Cache = json::from_slice(&data)?;
                    return Ok(curr.data);
                }
            }
            Err(anyhow!("timeout waiting for cached data"))
        }
        Err(err) => Err(err.into()),
    }
}

fn update<F>(dir: &Path, path: &Path, checksum: [u8; 20], f: F) -> Result<bool>
where
    F: FnOnce() -> Result<json::Value>,
{
    logger::init()?;
    let tmp = path.with_extension("tmp");
    if let Some(_guard) = fmutex::try_lock(dir)? {
        let data = f()?;
        let file = fs::File::create(&tmp)?;
        let modified = SystemTime::now();
        json::to_writer(
            &file,
            &Cache {
                checksum,
                modified,
                data,
            },
        )?;
        fs::rename(tmp, path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}
