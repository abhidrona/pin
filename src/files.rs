use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const SKIP_DIRS: &[&str] = &[
    ".git",
    ".pin",
    "target",
    "node_modules",
    "dist",
    "build",
    ".next",
    ".venv",
    "venv",
    "__pycache__",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMatch {
    pub path: String,
    pub score: i64,
}

pub fn search(root: &Path, query: &str, limit: usize) -> Vec<FileMatch> {
    let query = query.trim().trim_start_matches('@');
    let mut matches = Vec::new();
    for rel in project_files(root) {
        if query.is_empty() {
            matches.push(FileMatch {
                path: rel,
                score: 0,
            });
            continue;
        }
        if let Some(score) = fuzzy_score(query, &rel) {
            matches.push(FileMatch { path: rel, score });
        }
    }
    matches.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
    matches.truncate(limit);
    matches
}

pub fn resolve(root: &Path, raw: &str) -> Option<String> {
    let token = clean_token(raw);
    if token.is_empty() {
        return None;
    }
    let direct = normalize_relative_path(&token);
    if root.join(&direct).is_file() {
        return Some(direct);
    }
    search(root, &token, 1).into_iter().next().map(|m| m.path)
}

pub fn resolve_all(root: &Path, refs: &[String]) -> Vec<String> {
    let mut out = BTreeSet::new();
    for r in refs {
        if let Some(path) = resolve(root, r) {
            out.insert(path);
        }
    }
    out.into_iter().collect()
}

pub fn refs_in_text(root: &Path, text: &str) -> Vec<String> {
    let mut refs = BTreeSet::new();
    for token in at_tokens(text) {
        if let Some(path) = resolve(root, &token) {
            refs.insert(path);
        }
    }
    refs.into_iter().collect()
}

pub fn replace_refs_in_text(root: &Path, text: &str) -> String {
    let mut out = String::new();
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut i = 0;
    let mut last = 0;

    while i < chars.len() {
        if chars[i].1 != '@' {
            i += 1;
            continue;
        }
        let at_idx = chars[i].0;
        let start = at_idx + 1;
        i += 1;
        let mut end = start;
        while i < chars.len() && is_ref_char(chars[i].1) {
            end = chars[i].0 + chars[i].1.len_utf8();
            i += 1;
        }
        if end <= start {
            continue;
        }
        let token = &text[start..end];
        if let Some(path) = resolve(root, token) {
            out.push_str(&text[last..at_idx]);
            out.push('@');
            out.push_str(&path);
            last = end;
        }
    }
    out.push_str(&text[last..]);
    out
}

pub fn active_at_query(input: &str) -> Option<(usize, String)> {
    let mut start = None;
    for (idx, ch) in input.char_indices().rev() {
        if ch == '@' {
            start = Some(idx);
            break;
        }
        if ch.is_whitespace() {
            break;
        }
    }
    let start = start?;
    let query = &input[start + 1..];
    if query.chars().all(is_ref_char) {
        Some((start, query.to_string()))
    } else {
        None
    }
}

pub fn complete_at(input: &str, start: usize, path: &str) -> String {
    let mut out = String::new();
    out.push_str(&input[..start]);
    out.push('@');
    out.push_str(path);
    out
}

fn at_tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].1 != '@' {
            i += 1;
            continue;
        }
        let start = chars[i].0 + 1;
        i += 1;
        let mut end = start;
        while i < chars.len() && is_ref_char(chars[i].1) {
            end = chars[i].0 + chars[i].1.len_utf8();
            i += 1;
        }
        if end > start {
            out.push(text[start..end].to_string());
        }
    }
    out
}

fn is_ref_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/')
}

fn clean_token(raw: &str) -> String {
    raw.trim()
        .trim_start_matches('@')
        .trim_matches(|c: char| matches!(c, ',' | ';' | ':' | ')' | ']' | '}' | '\'' | '"'))
        .to_string()
}

fn normalize_relative_path(path: &str) -> String {
    let path = PathBuf::from(path);
    path.components()
        .filter_map(|c| match c {
            std::path::Component::Normal(part) => Some(part.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn project_files(root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                if SKIP_DIRS.contains(&name.as_str()) {
                    continue;
                }
                stack.push(path);
            } else if path.is_file() {
                if let Ok(rel) = path.strip_prefix(root) {
                    files.push(rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
    }
    files.sort();
    files
}

fn fuzzy_score(query: &str, candidate: &str) -> Option<i64> {
    let q = query.to_ascii_lowercase();
    let c = candidate.to_ascii_lowercase();
    if q.is_empty() {
        return Some(0);
    }
    if c == q {
        return Some(10_000 - candidate.len() as i64);
    }
    if c.contains(&q) {
        let pos = c.find(&q).unwrap_or(0) as i64;
        let filename_bonus = if candidate
            .rsplit('/')
            .next()
            .unwrap_or(candidate)
            .to_ascii_lowercase()
            .contains(&q)
        {
            500
        } else {
            0
        };
        return Some(8_000 + filename_bonus - pos - candidate.len() as i64);
    }

    let mut score = 0i64;
    let mut last_idx: Option<usize> = None;
    let mut search_from = 0usize;
    for qc in q.chars() {
        let tail = &c[search_from..];
        let found = tail.find(qc)?;
        let idx = search_from + found;
        score += 100;
        if let Some(prev) = last_idx {
            let gap = idx.saturating_sub(prev + 1) as i64;
            score -= gap.min(20);
        }
        if idx == 0
            || matches!(
                c.as_bytes().get(idx - 1).copied(),
                Some(b'/' | b'_' | b'-' | b'.')
            )
        {
            score += 35;
        }
        last_idx = Some(idx);
        search_from = idx + qc.len_utf8();
    }
    let filename = candidate
        .rsplit('/')
        .next()
        .unwrap_or(candidate)
        .to_ascii_lowercase();
    if fuzzy_contains(&q, &filename) {
        score += 250;
    }
    Some(score - candidate.len() as i64)
}

fn fuzzy_contains(query: &str, candidate: &str) -> bool {
    let mut it = candidate.chars();
    query.chars().all(|qc| it.any(|cc| cc == qc))
}
