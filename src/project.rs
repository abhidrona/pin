use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn detect_project_root(explicit_root: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(root) = explicit_root {
        return Ok(root.canonicalize().unwrap_or(root));
    }

    let cwd = std::env::current_dir().context("failed to read current directory")?;

    if let Some(root) = find_up(&cwd, ".pin/log.jsonl") {
        return Ok(root);
    }
    if let Some(root) = find_up(&cwd, ".git") {
        return Ok(root);
    }
    Ok(cwd)
}

fn find_up(start: &Path, marker: &str) -> Option<PathBuf> {
    let mut cur = Some(start);
    while let Some(path) = cur {
        if path.join(marker).exists() {
            return Some(path.to_path_buf());
        }
        cur = path.parent();
    }
    None
}

pub fn project_name(root: &Path) -> String {
    root.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project")
        .to_string()
}
