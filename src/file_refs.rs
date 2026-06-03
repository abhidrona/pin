use anyhow::{bail, Result};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMatch {
    pub path: String,
    pub score: i64,
}

pub fn search(root: &Path, query: &str, limit: usize) -> Result<Vec<FileMatch>> {
    let query = clean_query(query);
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let mut matches = Vec::new();
    for path in project_files(root)? {
        if let Some(score) = fuzzy_score(&query, &path) {
            matches.push(FileMatch { path, score });
        }
    }
    matches.sort_by(|a, b| match b.score.cmp(&a.score) {
        Ordering::Equal => a.path.cmp(&b.path),
        other => other,
    });
    matches.truncate(limit.max(1));
    Ok(matches)
}

pub fn resolve(root: &Path, raw: &[String]) -> Result<Vec<String>> {
    let mut out = BTreeSet::new();
    for value in raw {
        for part in value.split(',') {
            let q = clean_query(part);
            if q.is_empty() {
                continue;
            }
            if exact_file(root, &q) {
                out.insert(q);
                continue;
            }
            let matches = search(root, &q, 1)?;
            if let Some(best) = matches.first() {
                out.insert(best.path.clone());
            } else {
                bail!("No file matched '{q}'");
            }
        }
    }
    Ok(out.into_iter().collect())
}

pub fn extract_mentions(root: &Path, text: &str) -> Result<Vec<String>> {
    let mut queries = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '@' {
            i += 1;
            continue;
        }
        i += 1;
        let start = i;
        while i < chars.len() && is_ref_char(chars[i]) {
            i += 1;
        }
        if i > start {
            queries.push(chars[start..i].iter().collect::<String>());
        }
    }
    resolve(root, &queries)
}

pub fn merge(mut existing: Vec<String>, mut added: Vec<String>) -> Vec<String> {
    existing.append(&mut added);
    existing.sort();
    existing.dedup();
    existing
}

fn clean_query(query: &str) -> String {
    query
        .trim()
        .trim_start_matches('@')
        .trim_matches(|c: char| matches!(c, '"' | '\'' | ',' | ';' | ':' | ')' | ']' | '}'))
        .replace('\\', "/")
}

fn is_ref_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | '+')
}

fn exact_file(root: &Path, rel: &str) -> bool {
    let path = root.join(rel);
    path.is_file()
}

fn project_files(root: &Path) -> Result<Vec<String>> {
    let mut out = Vec::new();
    walk(root, root, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if should_skip_dir(&file_name) {
                continue;
            }
            walk(root, &path, out)?;
        } else if path.is_file() {
            if should_skip_file(&file_name) {
                continue;
            }
            if let Ok(rel) = path.strip_prefix(root) {
                out.push(normalize_path(rel));
            }
        }
    }
    Ok(())
}

fn normalize_path(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

fn should_skip_dir(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".pin"
            | "target"
            | "node_modules"
            | "dist"
            | "build"
            | ".next"
            | ".venv"
            | "venv"
            | "__pycache__"
            | ".idea"
            | ".vscode"
    )
}

fn should_skip_file(name: &str) -> bool {
    name.ends_with('~') || name.ends_with(".swp") || name == ".DS_Store"
}

fn fuzzy_score(query: &str, candidate: &str) -> Option<i64> {
    let q = query.to_ascii_lowercase();
    let c = candidate.to_ascii_lowercase();
    if q.is_empty() {
        return None;
    }
    if c == q {
        return Some(20_000);
    }
    if let Some(pos) = c.find(&q) {
        let basename = PathBuf::from(candidate)
            .file_name()
            .map(|s| s.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let mut score = 10_000 - pos as i64;
        if basename.contains(&q) {
            score += 2_000;
        }
        if pos == 0
            || c.as_bytes()
                .get(pos.wrapping_sub(1))
                .is_some_and(|b| is_boundary(*b as char))
        {
            score += 500;
        }
        return Some(score);
    }

    let mut score = 0_i64;
    let mut last_match: Option<usize> = None;
    let mut q_chars = q.chars();
    let mut wanted = q_chars.next()?;
    let mut matched = 0_usize;
    for (idx, ch) in c.chars().enumerate() {
        if ch == wanted {
            matched += 1;
            score += 100;
            if let Some(prev) = last_match {
                let gap = idx.saturating_sub(prev + 1) as i64;
                score -= gap.min(30);
                if gap == 0 {
                    score += 60;
                }
            }
            if idx == 0
                || c.chars()
                    .nth(idx.saturating_sub(1))
                    .is_some_and(is_boundary)
            {
                score += 80;
            }
            last_match = Some(idx);
            if let Some(next) = q_chars.next() {
                wanted = next;
            } else {
                let basename = PathBuf::from(candidate)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_default();
                if fuzzy_subsequence(&q, &basename) {
                    score += 500;
                }
                score -= candidate.len() as i64 / 4;
                return Some(score);
            }
        }
    }
    if matched == q.chars().count() {
        Some(score)
    } else {
        None
    }
}

fn fuzzy_subsequence(query: &str, candidate: &str) -> bool {
    let mut q = query.chars();
    let Some(mut want) = q.next() else {
        return false;
    };
    for ch in candidate.chars() {
        if ch == want {
            if let Some(next) = q.next() {
                want = next;
            } else {
                return true;
            }
        }
    }
    false
}

fn is_boundary(c: char) -> bool {
    matches!(c, '/' | '_' | '-' | '.' | ' ')
}
