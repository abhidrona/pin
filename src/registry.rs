use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEvent {
    pub v: u32,
    pub ts: DateTime<Utc>,
    pub op: String,
    pub name: String,
    pub root: PathBuf,
}

pub fn registry_path() -> Result<PathBuf> {
    let base = dirs::data_local_dir().context("failed to find local data directory")?;
    Ok(base.join("pin").join("registry.jsonl"))
}

pub fn register_project(name: &str, root: &Path) -> Result<()> {
    let path = registry_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let ev = RegistryEvent {
        v: 1,
        ts: Utc::now(),
        op: "project.seen".into(),
        name: name.into(),
        root: root.to_path_buf(),
    };
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    serde_json::to_writer(&mut file, &ev)?;
    file.write_all(b"\n")?;
    file.flush()?;
    file.sync_data().ok();
    Ok(())
}

pub fn load_projects() -> Result<Vec<RegistryEvent>> {
    let path = registry_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = OpenOptions::new().read(true).open(&path)?;
    let reader = BufReader::new(file);
    let mut latest: BTreeMap<PathBuf, RegistryEvent> = BTreeMap::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let ev: RegistryEvent = serde_json::from_str(&line)?;
        latest.insert(ev.root.clone(), ev);
    }
    Ok(latest.into_values().collect())
}
