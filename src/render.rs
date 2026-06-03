use crate::model::{AgentProgress, ProjectState, Status, Task};
use std::collections::BTreeMap;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub name: String,
    pub now: usize,
    pub review: usize,
    pub failed: usize,
    pub blocked: usize,
    pub todo: usize,
    pub verified: usize,
    pub cancelled: usize,
    pub total_active: usize,
}

pub fn list(state: &ProjectState, status: Option<Status>, active_only: bool) -> String {
    let mut out = String::new();
    out.push_str(&format!("Project: {}", state.meta.name));
    if let Some(branch) = git_branch(state) {
        out.push_str(&format!(" · {}", branch));
    }
    out.push('\n');
    out.push_str(&format!("Root: {}\n", state.meta.root.display()));
    if let Some(session) = visible_session(state) {
        out.push_str(&format!("Session: {}\n", session));
    }
    out.push('\n');

    for task in ordered_tasks(state) {
        if active_only && !task.status.is_active() {
            continue;
        }
        if let Some(s) = status {
            if !status_matches(task.status, s) {
                continue;
            }
        }
        out.push_str(&task_line(task));
        out.push('\n');
    }
    if state.tasks.is_empty() {
        out.push_str("No tasks. Add one with: pin add \"Your task\"\n");
    }
    out
}

pub fn show(state: &ProjectState) -> String {
    hud(state)
}

pub fn hud(state: &ProjectState) -> String {
    let mut out = String::new();
    out.push_str(&header(state));
    if let Some(session) = visible_session(state) {
        out.push_str(&format!(" · {}", session));
    }
    out.push_str("\n\n");

    section(&mut out, "NOW", state, &[Status::Doing]);
    section(
        &mut out,
        "REVIEW",
        state,
        &[Status::AgentDone, Status::NeedsReview],
    );
    section(&mut out, "FAILED", state, &[Status::Failed]);
    section(&mut out, "BLOCKED", state, &[Status::Blocked]);
    section(&mut out, "TODO", state, &[Status::Todo]);
    section(&mut out, "VERIFIED", state, &[Status::Verified]);
    section(&mut out, "CANCELLED", state, &[Status::Cancelled]);

    if !state.tasks.values().any(|t| t.status.is_active()) {
        out.push_str("No active tasks. Add one with: pin add \"Your task\"\n\n");
    }

    out.push_str("NEXT\n");
    out.push_str(&format!("{}\n", next_guidance(state)));
    out
}

pub fn task_detail(state: &ProjectState, id: u64) -> String {
    let Some(task) = state.tasks.get(&id) else {
        return format!("Task #{} not found\n", id);
    };
    let mut out = String::new();
    out.push_str(&format!("#{} {}\n", task.id, task.title));
    out.push_str(&format!("Status: {}\n", task.status.label()));
    out.push_str(&format!("Priority: {}\n", task.priority.as_str()));
    out.push_str(&format!(
        "Session: {}\n",
        task.session.as_deref().unwrap_or("default")
    ));
    if !task.tags.is_empty() {
        out.push_str(&format!(
            "Tags: {}\n",
            task.tags
                .iter()
                .map(|t| format!("#{t}"))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    if !task.files.is_empty() {
        out.push_str(&format!(
            "Files: {}\n",
            task.files
                .iter()
                .map(|f| format!("@{f}"))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    out.push('\n');

    out.push_str("Agent progress:\n");
    if task.agents.is_empty() {
        out.push_str("  none\n");
    } else {
        for progress in task.agents.values() {
            out.push_str(&agent_progress_line(progress));
        }
    }
    out.push('\n');

    out.push_str("Human notes:\n");
    let human_notes: Vec<_> = task.notes.iter().filter(|n| n.agent.is_none()).collect();
    if human_notes.is_empty() {
        out.push_str("  none\n");
    } else {
        for note in human_notes.iter().rev().take(8).rev() {
            out.push_str(&format!("  {}\n", note.body));
        }
    }
    out.push('\n');

    out.push_str("Next:\n");
    out.push_str(&format!("{}\n", next_for_task(task)));
    out
}

pub fn sessions(state: &ProjectState) -> String {
    let summaries = session_summaries(state);
    let mut out = String::new();
    out.push_str(&format!("{}\n\n", header(state)));
    if summaries.is_empty() {
        out.push_str("No active sessions. Add a task with: pin add \"Your task\"\n");
        return out;
    }
    out.push_str("ACTIVE SESSIONS\n");
    for s in summaries {
        out.push_str(&format!("  {:<22} {}\n", s.name, session_counts(&s)));
    }
    out
}

pub fn session_summaries(state: &ProjectState) -> Vec<SessionSummary> {
    let mut map: BTreeMap<String, SessionSummary> = BTreeMap::new();
    for task in state.tasks.values().filter(|t| t.status.is_active()) {
        let name = task.session.as_deref().unwrap_or("default").to_string();
        let e = map.entry(name.clone()).or_insert(SessionSummary {
            name,
            now: 0,
            review: 0,
            failed: 0,
            blocked: 0,
            todo: 0,
            verified: 0,
            cancelled: 0,
            total_active: 0,
        });
        e.total_active += 1;
        match task.status {
            Status::Doing => e.now += 1,
            Status::AgentDone | Status::NeedsReview => e.review += 1,
            Status::Failed => e.failed += 1,
            Status::Blocked => e.blocked += 1,
            Status::Todo => e.todo += 1,
            Status::Verified => e.verified += 1,
            Status::Cancelled => e.cancelled += 1,
            Status::Done => {}
        }
    }
    map.into_values().collect()
}

pub fn brief(state: &ProjectState) -> String {
    let mut out = String::new();
    out.push_str("PIN HANDOFF\n\n");
    out.push_str("Project:\n");
    out.push_str(&format!("- Name: {}\n", state.meta.name));
    if let Some(branch) = git_branch(state) {
        out.push_str(&format!("- Branch: {}\n", branch));
    }
    out.push_str(&format!("- Root: {}\n", state.meta.root.display()));
    if let Some(session) = visible_session(state) {
        out.push_str(&format!("- Session: {}\n", session));
    }
    out.push('\n');

    brief_section(
        &mut out,
        "Focus",
        state,
        &[
            Status::Doing,
            Status::Failed,
            Status::Blocked,
            Status::NeedsReview,
            Status::AgentDone,
        ],
    );
    brief_section(&mut out, "Open tasks", state, &[Status::Todo]);
    brief_section(
        &mut out,
        "Verified but not closed",
        state,
        &[Status::Verified],
    );
    brief_section(&mut out, "Cancelled", state, &[Status::Cancelled]);

    out.push_str("Recent notes:\n");
    let mut notes = Vec::new();
    for task in state.tasks.values() {
        for note in task.notes.iter().rev().take(2) {
            notes.push((note.created_at, task, note));
        }
    }
    notes.sort_by_key(|(ts, _, _)| ts.to_owned());
    notes.reverse();
    if notes.is_empty() {
        out.push_str("- none\n");
    } else {
        for (_, task, note) in notes.into_iter().take(8) {
            let who = note.agent.as_deref().unwrap_or(&note.author);
            out.push_str(&format!(
                "- #{} {} [{}]: {}\n",
                task.id, task.title, who, note.body
            ));
        }
    }
    out.push('\n');

    out.push_str("Next instruction:\n");
    out.push_str(&format!("{}\n", next_guidance(state)));
    out.push_str("Agents report progress. Humans pin task truth. Do not assume agent progress is verified. Keep changes scoped. After changes, provide exact verification steps.\n");
    out
}

fn header(state: &ProjectState) -> String {
    match git_branch(state) {
        Some(branch) => format!("{} · {}", state.meta.name, branch),
        None => state.meta.name.clone(),
    }
}

fn section(out: &mut String, title: &str, state: &ProjectState, statuses: &[Status]) {
    let tasks: Vec<_> = ordered_tasks(state)
        .into_iter()
        .filter(|t| statuses.contains(&t.status))
        .collect();
    if tasks.is_empty() {
        return;
    }
    out.push_str(title);
    out.push('\n');
    for t in tasks {
        let marker = if t.status == Status::Doing {
            "→"
        } else {
            " "
        };
        let suffix = task_suffix(t);
        out.push_str(&format!("{} #{} {}{}\n", marker, t.id, t.title, suffix));
    }
    out.push('\n');
}

fn brief_section(out: &mut String, title: &str, state: &ProjectState, statuses: &[Status]) {
    let tasks: Vec<_> = ordered_tasks(state)
        .into_iter()
        .filter(|t| statuses.contains(&t.status))
        .collect();
    out.push_str(title);
    out.push_str(":\n");
    if tasks.is_empty() {
        out.push_str("- none\n\n");
        return;
    }
    for t in tasks {
        out.push_str(&format!("- #{} {}\n", t.id, t.title));
        out.push_str(&format!("  Status: {}\n", t.status.label()));
        out.push_str(&format!("  Priority: {}\n", t.priority.as_str()));
        out.push_str(&format!(
            "  Session: {}\n",
            t.session.as_deref().unwrap_or("default")
        ));
        if !t.tags.is_empty() {
            out.push_str(&format!(
                "  Tags: {}\n",
                t.tags
                    .iter()
                    .map(|t| format!("#{t}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
        if !t.files.is_empty() {
            out.push_str(&format!(
                "  Files: {}\n",
                t.files
                    .iter()
                    .map(|f| format!("@{f}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
        if !t.agents.is_empty() {
            out.push_str("  Agent progress:\n");
            for a in t.agents.values() {
                out.push_str(&format!(
                    "    - {}: {}{}\n",
                    a.agent,
                    a.status.as_str(),
                    a.body
                        .as_ref()
                        .map(|b| format!(" — {}", b))
                        .unwrap_or_default()
                ));
            }
        }
        if let Some(note) = t.notes.iter().rev().find(|n| n.agent.is_none()) {
            out.push_str(&format!("  Latest human note: {}\n", note.body));
        }
    }
    out.push('\n');
}

pub fn task_line(task: &Task) -> String {
    format!(
        "[{}] {:<8} {:<6} {}{}{}",
        task.id,
        task.status.label(),
        task.priority.as_str(),
        task.title,
        task_suffix(task),
        agent_marker(task)
    )
}

fn task_suffix(task: &Task) -> String {
    let mut parts = Vec::new();
    if let Some(session) = &task.session {
        parts.push(format!("%{}", session));
    }
    for tag in &task.tags {
        parts.push(format!("#{}", tag));
    }
    if !task.files.is_empty() {
        parts.push(format!(
            "@{} file{}",
            task.files.len(),
            if task.files.len() == 1 { "" } else { "s" }
        ));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("  {}", parts.join(" "))
    }
}

fn agent_marker(task: &Task) -> String {
    match task.agents.len() {
        0 => String::new(),
        1 => format!("  · {}", task.agents.values().next().unwrap().agent),
        n => format!("  · {} agents", n),
    }
}

fn agent_progress_line(progress: &AgentProgress) -> String {
    match &progress.body {
        Some(body) => format!(
            "  {}: {} — {}\n",
            progress.agent,
            progress.status.as_str(),
            body
        ),
        None => format!("  {}: {}\n", progress.agent, progress.status.as_str()),
    }
}

fn session_counts(s: &SessionSummary) -> String {
    let mut parts = Vec::new();
    if s.now > 0 {
        parts.push(format!("{} now", s.now));
    }
    if s.review > 0 {
        parts.push(format!("{} review", s.review));
    }
    if s.failed > 0 {
        parts.push(format!("{} failed", s.failed));
    }
    if s.blocked > 0 {
        parts.push(format!("{} blocked", s.blocked));
    }
    if s.todo > 0 {
        parts.push(format!("{} todo", s.todo));
    }
    if s.verified > 0 {
        parts.push(format!("{} verified", s.verified));
    }
    if s.cancelled > 0 {
        parts.push(format!("{} cancelled", s.cancelled));
    }
    if parts.is_empty() {
        "no active tasks".to_string()
    } else {
        parts.join(" · ")
    }
}

fn ordered_tasks(state: &ProjectState) -> Vec<&Task> {
    let mut out = Vec::new();
    for statuses in [
        vec![Status::Doing],
        vec![Status::AgentDone, Status::NeedsReview],
        vec![Status::Failed],
        vec![Status::Blocked],
        vec![Status::Todo],
        vec![Status::Verified],
        vec![Status::Cancelled],
        vec![Status::Done],
    ] {
        for task in state
            .tasks
            .values()
            .filter(|t| statuses.contains(&t.status))
        {
            out.push(task);
        }
    }
    out
}

fn next_guidance(state: &ProjectState) -> String {
    if let Some(t) = first_with_status(state, Status::Failed) {
        return format!("Fix failed task #{} first: {}. Inspect task details with `pin show {}` before asking an agent to continue.", t.id, t.title, t.id);
    }
    if let Some(t) = first_with_status(state, Status::Blocked) {
        return format!(
            "Unblock task #{} first: {}. Resolve the blocker or choose another unblocked task.",
            t.id, t.title
        );
    }
    if let Some(t) = first_with_status(state, Status::NeedsReview)
        .or_else(|| first_with_status(state, Status::AgentDone))
    {
        return format!(
            "Review task #{}: {}. Check agent progress in `pin show {}` and pin verified/failed.",
            t.id, t.title, t.id
        );
    }
    if let Some(t) = first_with_status(state, Status::Doing) {
        return format!(
            "Continue active task #{}: {}. Record agent progress with `pin ag {}` when useful.",
            t.id, t.title, t.id
        );
    }
    if let Some(t) = first_with_status(state, Status::Todo) {
        return format!("Start task #{}: {}.", t.id, t.title);
    }
    if let Some(t) = first_with_status(state, Status::Verified) {
        return format!(
            "Task #{} is verified. Close it with `pin done {}` or start the next task.",
            t.id, t.id
        );
    }
    "No active work. Add a task with `pin add \"...\"`.".to_string()
}

fn next_for_task(task: &Task) -> String {
    match task.status {
        Status::Failed => "Fix the failed task first. Inspect the human failure note and ask the agent for a scoped fix.".to_string(),
        Status::Blocked => "Resolve the blocker or switch to another unblocked task.".to_string(),
        Status::NeedsReview | Status::AgentDone => "Review agent progress, run the smallest verification, then pin verified or failed.".to_string(),
        Status::Doing => "Continue the task. Record agent progress when Claude/Codex reports meaningful progress.".to_string(),
        Status::Todo => "Start the task when ready.".to_string(),
        Status::Verified => format!("Close it with `pin done {}` when ready, or `pin cancel {}` if it no longer applies.", task.id, task.id),
        Status::Cancelled => "Cancelled. No further action needed unless you reopen it by starting/reviewing it again.".to_string(),
        Status::Done => "Already closed.".to_string(),
    }
}

fn first_with_status(state: &ProjectState, status: Status) -> Option<&Task> {
    state
        .tasks
        .values()
        .find(|t| status_matches(t.status, status))
}

fn status_matches(actual: Status, wanted: Status) -> bool {
    actual == wanted || (wanted == Status::AgentDone && actual == Status::NeedsReview)
}

fn visible_session(state: &ProjectState) -> Option<String> {
    let mut sessions: Vec<String> = state
        .tasks
        .values()
        .filter_map(|t| t.session.clone())
        .collect();
    sessions.sort();
    sessions.dedup();
    if sessions.len() == 1 {
        sessions.pop()
    } else {
        None
    }
}

fn git_branch(state: &ProjectState) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(&state.meta.root)
        .arg("branch")
        .arg("--show-current")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        None
    } else {
        Some(branch)
    }
}
