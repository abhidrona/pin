use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Todo,
    Doing,
    #[value(alias = "reported", alias = "claim", alias = "claimed")]
    AgentDone,
    #[value(alias = "review")]
    NeedsReview,
    Verified,
    Failed,
    Blocked,
    Cancelled,
    #[value(alias = "complete", alias = "completed", alias = "closed")]
    Done,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Todo => "todo",
            Status::Doing => "doing",
            Status::AgentDone => "agent-done",
            Status::NeedsReview => "needs-review",
            Status::Verified => "verified",
            Status::Failed => "failed",
            Status::Blocked => "blocked",
            Status::Cancelled => "cancelled",
            Status::Done => "done",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Status::AgentDone => "review",
            Status::NeedsReview => "review",
            _ => self.as_str(),
        }
    }

    pub fn is_active(self) -> bool {
        !matches!(self, Status::Done | Status::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum AgentStatus {
    Assigned,
    Working,
    Reported,
    NeedsInput,
    Failed,
    Stopped,
}

impl AgentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            AgentStatus::Assigned => "assigned",
            AgentStatus::Working => "working",
            AgentStatus::Reported => "reported",
            AgentStatus::NeedsInput => "needs-input",
            AgentStatus::Failed => "failed",
            AgentStatus::Stopped => "stopped",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Priority {
    Low,
    Normal,
    High,
    Urgent,
}

impl Priority {
    pub fn as_str(self) -> &'static str {
        match self {
            Priority::Low => "low",
            Priority::Normal => "normal",
            Priority::High => "high",
            Priority::Urgent => "urgent",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub v: u32,
    pub name: String,
    pub root: PathBuf,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProgress {
    pub agent: String,
    pub status: AgentStatus,
    pub body: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub status: Status,
    pub priority: Priority,
    /// Workstream inside the current folder/worktree. Defaults to "default".
    pub session: Option<String>,
    /// Optional initial owner/agent for compatibility with older logs.
    pub agent: Option<String>,
    pub tags: Vec<String>,
    /// Files referenced by this task through @path tokens or explicit refs.
    pub files: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub notes: Vec<Note>,
    /// Latest progress reported by each agent. This is separate from human-owned task status.
    pub agents: BTreeMap<String, AgentProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub created_at: DateTime<Utc>,
    pub author: String,
    pub agent: Option<String>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub v: u32,
    pub ts: DateTime<Utc>,
    pub op: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_status: Option<AgentStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<Priority>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
}

impl Event {
    pub fn now(op: impl Into<String>) -> Self {
        Self {
            v: 1,
            ts: Utc::now(),
            op: op.into(),
            id: None,
            title: None,
            status: None,
            agent_status: None,
            priority: None,
            agent: None,
            body: None,
            session: None,
            tags: None,
            files: None,
        }
    }
}

fn add_files(existing: &mut Vec<String>, new_files: Vec<String>) {
    for file in new_files {
        if !file.trim().is_empty() && !existing.contains(&file) {
            existing.push(file);
        }
    }
    existing.sort();
}

#[derive(Debug, Clone)]
pub struct ProjectState {
    pub meta: ProjectMeta,
    pub next_id: u64,
    pub tasks: BTreeMap<u64, Task>,
}

impl ProjectState {
    pub fn from_events(meta: ProjectMeta, events: &[Event]) -> Self {
        let mut next_id = 1;
        let mut tasks: BTreeMap<u64, Task> = BTreeMap::new();

        for ev in events {
            match ev.op.as_str() {
                "task.add" => {
                    if let (Some(id), Some(title)) = (ev.id, ev.title.clone()) {
                        next_id = next_id.max(id + 1);
                        let agent = ev.agent.clone();
                        let mut agents = BTreeMap::new();
                        if let Some(a) = agent.clone() {
                            agents.insert(
                                a.clone(),
                                AgentProgress {
                                    agent: a,
                                    status: AgentStatus::Assigned,
                                    body: None,
                                    updated_at: ev.ts,
                                },
                            );
                        }
                        tasks.insert(
                            id,
                            Task {
                                id,
                                title,
                                status: Status::Todo,
                                priority: ev.priority.unwrap_or(Priority::Normal),
                                session: ev.session.clone().or_else(|| Some("default".to_string())),
                                agent,
                                tags: ev.tags.clone().unwrap_or_default(),
                                files: ev.files.clone().unwrap_or_default(),
                                created_at: ev.ts,
                                updated_at: ev.ts,
                                notes: Vec::new(),
                                agents,
                            },
                        );
                    }
                }
                "task.title" => {
                    if let (Some(id), Some(title)) = (ev.id, ev.title.clone()) {
                        if let Some(t) = tasks.get_mut(&id) {
                            t.title = title;
                            add_files(&mut t.files, ev.files.clone().unwrap_or_default());
                            t.updated_at = ev.ts;
                        }
                    }
                }
                "task.status" => {
                    if let (Some(id), Some(status)) = (ev.id, ev.status) {
                        if let Some(t) = tasks.get_mut(&id) {
                            t.status = status;
                            add_files(&mut t.files, ev.files.clone().unwrap_or_default());
                            t.updated_at = ev.ts;
                        }
                    }
                    if let (Some(id), Some(body)) = (ev.id, ev.body.clone()) {
                        if let Some(t) = tasks.get_mut(&id) {
                            t.notes.push(Note {
                                created_at: ev.ts,
                                author: "human".into(),
                                agent: None,
                                body,
                            });
                            add_files(&mut t.files, ev.files.clone().unwrap_or_default());
                            t.updated_at = ev.ts;
                        }
                    }
                }
                "task.note" => {
                    if let (Some(id), Some(body)) = (ev.id, ev.body.clone()) {
                        if let Some(t) = tasks.get_mut(&id) {
                            t.notes.push(Note {
                                created_at: ev.ts,
                                author: if ev.agent.is_some() {
                                    "agent".into()
                                } else {
                                    "human".into()
                                },
                                agent: ev.agent.clone(),
                                body,
                            });
                            add_files(&mut t.files, ev.files.clone().unwrap_or_default());
                            t.updated_at = ev.ts;
                        }
                    }
                }
                "agent.progress" => {
                    if let (Some(id), Some(agent), Some(status)) =
                        (ev.id, ev.agent.clone(), ev.agent_status)
                    {
                        if let Some(t) = tasks.get_mut(&id) {
                            t.agents.insert(
                                agent.clone(),
                                AgentProgress {
                                    agent: agent.clone(),
                                    status,
                                    body: ev.body.clone(),
                                    updated_at: ev.ts,
                                },
                            );
                            if let Some(body) = ev.body.clone() {
                                t.notes.push(Note {
                                    created_at: ev.ts,
                                    author: "agent".into(),
                                    agent: Some(agent),
                                    body,
                                });
                            }
                            add_files(&mut t.files, ev.files.clone().unwrap_or_default());
                            t.updated_at = ev.ts;
                        }
                    }
                }
                "task.tags.add" => {
                    if let (Some(id), Some(tags)) = (ev.id, ev.tags.clone()) {
                        if let Some(t) = tasks.get_mut(&id) {
                            for tag in tags {
                                if !t.tags.contains(&tag) {
                                    t.tags.push(tag);
                                }
                            }
                            t.tags.sort();
                            t.updated_at = ev.ts;
                        }
                    }
                }
                "task.tags.remove" => {
                    if let (Some(id), Some(tags)) = (ev.id, ev.tags.clone()) {
                        if let Some(t) = tasks.get_mut(&id) {
                            t.tags.retain(|tag| !tags.contains(tag));
                            t.updated_at = ev.ts;
                        }
                    }
                }
                "task.files.add" => {
                    if let Some(id) = ev.id {
                        if let Some(t) = tasks.get_mut(&id) {
                            add_files(&mut t.files, ev.files.clone().unwrap_or_default());
                            t.updated_at = ev.ts;
                        }
                    }
                }
                "task.files.remove" => {
                    if let (Some(id), Some(files)) = (ev.id, ev.files.clone()) {
                        if let Some(t) = tasks.get_mut(&id) {
                            t.files.retain(|file| !files.contains(file));
                            t.updated_at = ev.ts;
                        }
                    }
                }
                _ => {}
            }
        }

        Self {
            meta,
            next_id,
            tasks,
        }
    }

    pub fn filtered(&self, session: Option<&str>, tags: &[String]) -> Self {
        let tasks = self
            .tasks
            .iter()
            .filter(|(_, task)| {
                if let Some(session) = session {
                    if task.session.as_deref().unwrap_or("default") != session {
                        return false;
                    }
                }
                tags.iter().all(|tag| task.tags.contains(tag))
            })
            .map(|(id, task)| (*id, task.clone()))
            .collect();
        Self {
            meta: self.meta.clone(),
            next_id: self.next_id,
            tasks,
        }
    }
}
