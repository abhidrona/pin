use crate::file_refs;
use crate::model::{AgentStatus, Event, Priority, Status};
use crate::storage::{append_event, load_state};
use anyhow::{bail, Result};
use std::path::Path;

pub fn add(
    root: &Path,
    title: String,
    priority: Priority,
    agent: Option<String>,
    tags: Vec<String>,
    session: Option<String>,
    files: Vec<String>,
) -> Result<u64> {
    let title = title.trim().to_string();
    if title.is_empty() {
        bail!("Task title cannot be empty");
    }
    let state = load_state(root)?;
    let id = state.next_id;
    let mut ev = Event::now("task.add");
    ev.id = Some(id);
    ev.title = Some(title.clone());
    ev.priority = Some(priority);
    ev.agent = clean_optional(agent);
    ev.session = clean_optional(session).or_else(|| Some("default".to_string()));
    let tags = normalize_tags(tags);
    if !tags.is_empty() {
        ev.tags = Some(tags);
    }
    let mut files = file_refs::resolve(root, &files)?;
    files = file_refs::merge(files, file_refs::extract_mentions(root, &title)?);
    if !files.is_empty() {
        ev.files = Some(files);
    }
    append_event(root, &ev)?;
    Ok(id)
}

pub fn set_title(root: &Path, id: u64, title: String) -> Result<()> {
    let title = title.trim().to_string();
    if title.is_empty() {
        bail!("Task title cannot be empty");
    }
    require_task(root, id)?;
    let mut ev = Event::now("task.title");
    ev.id = Some(id);
    let files = file_refs::extract_mentions(root, &title)?;
    ev.title = Some(title);
    if !files.is_empty() {
        ev.files = Some(files);
    }
    append_event(root, &ev)
}

pub fn status(root: &Path, id: u64, status: Status, body: Option<String>) -> Result<()> {
    require_task(root, id)?;
    if matches!(status, Status::Failed | Status::Blocked)
        && body.as_deref().unwrap_or("").trim().is_empty()
    {
        bail!("{} reason is required", status.as_str());
    }
    let mut ev = Event::now("task.status");
    ev.id = Some(id);
    ev.status = Some(status);
    ev.body = body.map(|b| b.trim().to_string()).filter(|b| !b.is_empty());
    if let Some(body) = ev.body.as_deref() {
        let files = file_refs::extract_mentions(root, body)?;
        if !files.is_empty() {
            ev.files = Some(files);
        }
    }
    append_event(root, &ev)
}

pub fn note(root: &Path, id: u64, body: String, agent: Option<String>) -> Result<()> {
    let body = body.trim().to_string();
    if body.is_empty() {
        bail!("Note cannot be empty");
    }
    require_task(root, id)?;
    let mut ev = Event::now("task.note");
    ev.id = Some(id);
    let files = file_refs::extract_mentions(root, &body)?;
    ev.body = Some(body);
    ev.agent = clean_optional(agent);
    if !files.is_empty() {
        ev.files = Some(files);
    }
    append_event(root, &ev)
}

pub fn add_tags(root: &Path, id: u64, tags: Vec<String>) -> Result<()> {
    let tags = normalize_tags(tags);
    if tags.is_empty() {
        bail!("At least one tag is required");
    }
    require_task(root, id)?;
    let mut ev = Event::now("task.tags.add");
    ev.id = Some(id);
    ev.tags = Some(tags);
    append_event(root, &ev)
}

pub fn remove_tags(root: &Path, id: u64, tags: Vec<String>) -> Result<()> {
    let tags = normalize_tags(tags);
    if tags.is_empty() {
        bail!("At least one tag is required");
    }
    require_task(root, id)?;
    let mut ev = Event::now("task.tags.remove");
    ev.id = Some(id);
    ev.tags = Some(tags);
    append_event(root, &ev)
}

pub fn agent_progress(
    root: &Path,
    id: u64,
    agent: String,
    status: AgentStatus,
    body: String,
) -> Result<()> {
    let agent = agent.trim().to_string();
    if agent.is_empty() {
        bail!("Agent name cannot be empty");
    }
    let body = body.trim().to_string();
    if body.is_empty() {
        bail!("Agent progress note cannot be empty");
    }
    require_task(root, id)?;
    let mut ev = Event::now("agent.progress");
    ev.id = Some(id);
    ev.agent = Some(agent);
    ev.agent_status = Some(status);
    let files = file_refs::extract_mentions(root, &body)?;
    ev.body = Some(body);
    if !files.is_empty() {
        ev.files = Some(files);
    }
    append_event(root, &ev)
}

/// Human-triggered agent report.
///
/// This records what the agent claims it changed, then moves the human-owned
/// task status to review. It deliberately does not pin the task verified/done.
pub fn agent_report(root: &Path, id: u64, agent: String, body: String) -> Result<()> {
    let agent = agent.trim().to_string();
    if agent.is_empty() {
        bail!("Agent name cannot be empty");
    }
    let body = body.trim().to_string();
    if body.is_empty() {
        bail!("Agent report cannot be empty");
    }
    require_task(root, id)?;
    agent_progress(root, id, agent, AgentStatus::Reported, body)?;
    status(root, id, Status::NeedsReview, None)
}

pub fn add_files(root: &Path, id: u64, files: Vec<String>) -> Result<()> {
    let files = file_refs::resolve(root, &files)?;
    if files.is_empty() {
        bail!("At least one file reference is required");
    }
    require_task(root, id)?;
    let mut ev = Event::now("task.files.add");
    ev.id = Some(id);
    ev.files = Some(files);
    append_event(root, &ev)
}

pub fn remove_files(root: &Path, id: u64, files: Vec<String>) -> Result<()> {
    let files = file_refs::resolve(root, &files)?;
    if files.is_empty() {
        bail!("At least one file reference is required");
    }
    require_task(root, id)?;
    let mut ev = Event::now("task.files.remove");
    ev.id = Some(id);
    ev.files = Some(files);
    append_event(root, &ev)
}

pub fn normalize_tags(raw: Vec<String>) -> Vec<String> {
    let mut tags = Vec::new();
    for value in raw {
        for part in value.split(',') {
            let tag = part.trim().trim_start_matches('#').to_ascii_lowercase();
            if tag.is_empty() {
                continue;
            }
            if tag
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                && !tags.contains(&tag)
            {
                tags.push(tag);
            }
        }
    }
    tags.sort();
    tags
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn require_task(root: &Path, id: u64) -> Result<()> {
    let state = load_state(root)?;
    if state.tasks.contains_key(&id) {
        Ok(())
    } else {
        bail!("Task #{} not found", id)
    }
}
