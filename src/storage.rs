use crate::model::{Event, ProjectMeta, ProjectState};
use crate::project::project_name;
use anyhow::{bail, Context, Result};
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub const MARK_DIR: &str = ".pin";
pub const META_FILE: &str = "meta.json";
pub const LOG_FILE: &str = "log.jsonl";

pub fn init_project(root: &Path) -> Result<bool> {
    let dir = root.join(MARK_DIR);
    let meta_path = dir.join(META_FILE);
    let log_path = dir.join(LOG_FILE);

    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;

    let created = !meta_path.exists();
    if created {
        let meta = ProjectMeta {
            v: 1,
            name: project_name(root),
            root: root.to_path_buf(),
            created_at: Utc::now(),
        };
        atomic_write_json(&meta_path, &meta)?;
    }

    if !log_path.exists() {
        OpenOptions::new().create(true).append(true).open(&log_path)?;
    }

    Ok(created)
}


pub fn load_meta(root: &Path) -> Result<ProjectMeta> {
    let path = root.join(MARK_DIR).join(META_FILE);
    let data = fs::read_to_string(&path)
        .with_context(|| "pin is not initialized in this project. Run: pin init".to_string())?;
    serde_json::from_str(&data).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn load_events(root: &Path) -> Result<Vec<Event>> {
    let path = root.join(MARK_DIR).join(LOG_FILE);
    if !path.exists() {
        bail!("pin is not initialized in this project. Run: pin init");
    }
    let file = OpenOptions::new().read(true).open(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() { continue; }
        let ev: Event = serde_json::from_str(&line)
            .with_context(|| format!("failed to parse {} at line {}", path.display(), idx + 1))?;
        events.push(ev);
    }
    Ok(events)
}

pub fn load_state(root: &Path) -> Result<ProjectState> {
    let meta = load_meta(root)?;
    let events = load_events(root)?;
    Ok(ProjectState::from_events(meta, &events))
}

pub fn append_event(root: &Path, ev: &Event) -> Result<()> {
    let path = root.join(MARK_DIR).join(LOG_FILE);
    let mut file = OpenOptions::new().create(true).append(true).open(&path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    serde_json::to_writer(&mut file, ev)?;
    file.write_all(b"\n")?;
    file.flush()?;
    file.sync_data().ok();
    Ok(())
}

fn atomic_write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let tmp = tmp_path(path);
    let data = serde_json::to_vec_pretty(value)?;
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut tmp = path.to_path_buf();
    tmp.set_extension("tmp");
    tmp
}
