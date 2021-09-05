use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use once_cell::sync::Lazy;
use powerpack::env;
use serde::{Deserialize, Serialize};

use crate::detach;
use crate::github;

const UPDATE_INTERVAL: Duration = Duration::from_secs(60 * 60);

static CACHE_DIR: Lazy<PathBuf> =
    Lazy::new(|| env::workflow_cache().expect("failed to get workflow cache directory"));

#[derive(Debug, Deserialize, Serialize)]
struct Cache {
    modified: SystemTime,
    repos: Vec<github::Repo>,
}

impl Cache {
    fn new(repos: Vec<github::Repo>) -> Self {
        Self {
            modified: SystemTime::now(),
            repos,
        }
    }

    fn empty() -> Self {
        Self {
            modified: SystemTime::now(),
            repos: Vec::new(),
        }
    }

    fn dump(&self) -> Result<()> {
        fs::create_dir_all(&*CACHE_DIR)?;
        let path = CACHE_DIR.join("cache.json");
        let bytes = serde_json::to_vec(self)?;
        Ok(fs::write(path, &bytes)?)
    }
}

fn try_load() -> Result<Cache> {
    let path = CACHE_DIR.join("cache.json");
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn update() -> Result<()> {
    let cache = Cache::new(github::repos()?);
    cache.dump()?;
    Ok(())
}

fn check_and_load() -> Result<Cache> {
    match try_load() {
        Ok(cache) => {
            let now = SystemTime::now();
            if now.duration_since(cache.modified)? > UPDATE_INTERVAL {
                detach::child(update)?;
            }
            Ok(cache)
        }
        Err(_) => {
            detach::child(update)?;
            Ok(Cache::empty())
        }
    }
}

pub fn repos() -> Result<Vec<github::Repo>> {
    check_and_load().map(|cache| cache.repos)
}
