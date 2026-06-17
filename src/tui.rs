use crate::model::{AgentStatus, Priority, ProjectState, Status, Task};
use crate::{files, ops, storage};
use anyhow::Result;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event as CtEvent, KeyCode};
use crossterm::style::{Attribute, Print, SetAttribute};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn run(root: PathBuf, session: Option<String>, filter_tags: Vec<String>) -> Result<()> {
    let mut stdout = io::stdout();
    let _guard = TerminalGuard::enter(&mut stdout)?;

    let mut selected: usize = 0;
    let mut show_help = false;
    let mut details = false;

    loop {
        let state = storage::load_state(&root)?.filtered(session.as_deref(), &filter_tags);
        let visible_ids: Vec<u64> = ordered_visible(&state).into_iter().map(|t| t.id).collect();
        if selected >= visible_ids.len() && !visible_ids.is_empty() {
            selected = visible_ids.len() - 1;
        }
        if details {
            draw_detail(
                &mut stdout,
                &state,
                visible_ids.get(selected).copied(),
                show_help,
            )?;
        } else {
            draw_list(&mut stdout, &state, selected, show_help)?;
        }

        if event::poll(Duration::from_millis(250))? {
            if let CtEvent::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Esc => {
                        if details {
                            details = false;
                        } else {
                            break;
                        }
                    }
                    KeyCode::Enter => {
                        if !visible_ids.is_empty() {
                            details = !details;
                        }
                    }
                    KeyCode::Char('?') => show_help = !show_help,
                    KeyCode::Down | KeyCode::Char('j') if !details => {
                        if selected + 1 < visible_ids.len() {
                            selected += 1;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') if !details => {
                        selected = selected.saturating_sub(1);
                    }
                    KeyCode::Char('a') if !details => {
                        let title = prompt(&root, "Add task", "")?;
                        if !title.trim().is_empty() {
                            let tags = prompt(&root, "Tags optional, comma-separated", "")?;
                            let _ = ops::add(
                                &root,
                                title,
                                Priority::Normal,
                                None,
                                vec![tags],
                                session.clone(),
                                Vec::new(),
                            );
                        }
                    }
                    KeyCode::Char('e') => {
                        if let Some(id) = visible_ids.get(selected).copied() {
                            let existing = state
                                .tasks
                                .get(&id)
                                .map(|task| task.title.clone())
                                .unwrap_or_default();
                            let title = prompt(&root, "Edit title", &existing)?;
                            if !title.trim().is_empty() {
                                let _ = ops::set_title(&root, id, title);
                            }
                        }
                    }
                    KeyCode::Char('s') => {
                        set_selected(&root, &visible_ids, selected, Status::Doing, None)
                    }
                    KeyCode::Char('r') => {
                        set_selected(&root, &visible_ids, selected, Status::NeedsReview, None)
                    }
                    KeyCode::Char('v') => {
                        if let Some(id) = visible_ids.get(selected).copied() {
                            let msg = prompt(&root, "Verified note optional", "")?;
                            let body = if msg.trim().is_empty() {
                                None
                            } else {
                                Some(msg)
                            };
                            let _ = ops::status(&root, id, Status::Verified, body);
                        }
                    }
                    KeyCode::Char('f') => {
                        if let Some(id) = visible_ids.get(selected).copied() {
                            let msg = prompt(&root, "Failure reason", "")?;
                            let _ = ops::status(&root, id, Status::Failed, Some(msg));
                        }
                    }
                    KeyCode::Char('b') => {
                        if let Some(id) = visible_ids.get(selected).copied() {
                            let msg = prompt(&root, "Blocked reason", "")?;
                            let _ = ops::status(&root, id, Status::Blocked, Some(msg));
                        }
                    }
                    KeyCode::Char('c') => {
                        set_selected(&root, &visible_ids, selected, Status::Cancelled, None)
                    }
                    KeyCode::Char('d') => {
                        set_selected(&root, &visible_ids, selected, Status::Done, None)
                    }
                    KeyCode::Char('n') => {
                        if let Some(id) = visible_ids.get(selected).copied() {
                            let msg = prompt(&root, "Human note", "")?;
                            if !msg.trim().is_empty() {
                                let _ = ops::note(&root, id, msg, None);
                            }
                        }
                    }
                    KeyCode::Char('t') => {
                        if let Some(id) = visible_ids.get(selected).copied() {
                            let tags = prompt(&root, "Add tags, comma-separated", "")?;
                            let _ = ops::add_tags(&root, id, vec![tags]);
                        }
                    }
                    KeyCode::Char('@') => {
                        if let Some(id) = visible_ids.get(selected).copied() {
                            let query = prompt(&root, "Attach file", "@")?;
                            let query = query.trim().trim_start_matches('@').to_string();
                            if !query.is_empty() {
                                let _ = ops::add_files(&root, id, vec![query]);
                            }
                        }
                    }
                    KeyCode::Char('A') | KeyCode::Char('G') => {
                        if let Some(id) = visible_ids.get(selected).copied() {
                            let agent = prompt(&root, "Agent name", "")?;
                            let status_raw = prompt(
                                &root,
                                "Agent status (assigned|working|reported|needs-input|failed|stopped)",
                                "reported",
                            )?;
                            let status =
                                parse_agent_status(&status_raw).unwrap_or(AgentStatus::Reported);
                            let msg = prompt(&root, "Agent progress", "")?;
                            if status == AgentStatus::Reported {
                                let _ = ops::agent_report(&root, id, agent, msg);
                            } else {
                                let _ = ops::agent_progress(&root, id, agent, status, msg);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter(stdout: &mut io::Stdout) -> Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen, Hide)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = execute!(stdout, Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

fn parse_agent_status(s: &str) -> Option<AgentStatus> {
    match s.trim() {
        "assigned" => Some(AgentStatus::Assigned),
        "working" => Some(AgentStatus::Working),
        "reported" => Some(AgentStatus::Reported),
        "needs-input" | "needs_input" => Some(AgentStatus::NeedsInput),
        "failed" => Some(AgentStatus::Failed),
        "stopped" => Some(AgentStatus::Stopped),
        _ => None,
    }
}

fn set_selected(root: &Path, ids: &[u64], selected: usize, status: Status, body: Option<String>) {
    if let Some(id) = ids.get(selected).copied() {
        let _ = ops::status(root, id, status, body);
    }
}

fn draw_list(
    stdout: &mut io::Stdout,
    state: &ProjectState,
    selected: usize,
    show_help: bool,
) -> Result<()> {
    let (cols, rows) = terminal::size()?;
    let w = cols.clamp(56, 104);
    let h = rows.clamp(16, 32);
    let x = (cols.saturating_sub(w)) / 2;
    let y = (rows.saturating_sub(h)) / 2;
    let visible = ordered_visible(state);
    let selected_id = visible.get(selected).map(|t| t.id);

    queue!(stdout, Clear(ClearType::All))?;
    draw_box(stdout, x, y, w, h)?;
    queue!(
        stdout,
        MoveTo(x + 2, y + 1),
        SetAttribute(Attribute::Bold),
        Print(&state.meta.name),
        SetAttribute(Attribute::Reset)
    )?;
    queue!(
        stdout,
        MoveTo(x + w - 40, y + 1),
        Print("q close  Enter details  a add  @ file  A agent")
    )?;
    queue!(
        stdout,
        MoveTo(x + 2, y + 2),
        Print(trunc(
            &format!("{} · {}", state.meta.root.display(), visible_session(state)),
            (w - 4) as usize
        ))
    )?;

    let mut line = y + 4;
    if visible.is_empty() {
        queue!(
            stdout,
            MoveTo(x + 2, line),
            Print("No active tasks. Press 'a' to add one.")
        )?;
    } else {
        for (title, statuses) in sections() {
            let tasks: Vec<_> = state
                .tasks
                .values()
                .filter(|t| statuses.contains(&t.status))
                .collect();
            if tasks.is_empty() {
                continue;
            }
            if line >= y + h - 4 {
                break;
            }
            queue!(
                stdout,
                MoveTo(x + 2, line),
                SetAttribute(Attribute::Bold),
                Print(title),
                SetAttribute(Attribute::Reset)
            )?;
            line += 1;
            for task in tasks {
                if line >= y + h - 4 {
                    break;
                }
                queue!(stdout, MoveTo(x + 2, line))?;
                if selected_id == Some(task.id) {
                    queue!(stdout, SetAttribute(Attribute::Reverse))?;
                }
                let marker = if task.status == Status::Doing {
                    "→"
                } else {
                    " "
                };
                let mut suffix = String::new();
                if let Some(s) = &task.session {
                    suffix.push_str(&format!(" %{}", s));
                }
                for tag in &task.tags {
                    suffix.push_str(&format!(" #{}", tag));
                }
                if !task.agents.is_empty() {
                    suffix.push_str(&format!(" · {}", agent_compact(task)));
                }
                let text = trunc(
                    &format!("{} #{} {}{}", marker, task.id, task.title, suffix),
                    (w - 4) as usize,
                );
                queue!(
                    stdout,
                    Print(format!("{:<width$}", text, width = (w - 4) as usize))
                )?;
                if selected_id == Some(task.id) {
                    queue!(stdout, SetAttribute(Attribute::Reset))?;
                }
                line += 1;
            }
            line += 1;
        }
    }
    let help = if show_help {
        "j/k move · Enter details · a add · A agent · s start · r review · v verify · f fail · b block · c cancel · d done · n note · t tags · ? hide"
    } else {
        "? help · Enter details · a add · A agent · n note · v verify · f fail · c cancel · q close"
    };
    queue!(
        stdout,
        MoveTo(x + 2, y + h - 2),
        Print(trunc(help, (w - 4) as usize))
    )?;
    stdout.flush()?;
    Ok(())
}

fn draw_detail(
    stdout: &mut io::Stdout,
    state: &ProjectState,
    id: Option<u64>,
    show_help: bool,
) -> Result<()> {
    let (cols, rows) = terminal::size()?;
    let w = cols.clamp(56, 104);
    let h = rows.clamp(16, 32);
    let x = (cols.saturating_sub(w)) / 2;
    let y = (rows.saturating_sub(h)) / 2;
    queue!(stdout, Clear(ClearType::All))?;
    draw_box(stdout, x, y, w, h)?;
    let Some(id) = id else {
        return Ok(());
    };
    let Some(task) = state.tasks.get(&id) else {
        return Ok(());
    };
    queue!(
        stdout,
        MoveTo(x + 2, y + 1),
        SetAttribute(Attribute::Bold),
        Print(format!(
            "#{} {}",
            task.id,
            trunc(&task.title, (w - 12) as usize)
        )),
        SetAttribute(Attribute::Reset)
    )?;
    queue!(
        stdout,
        MoveTo(x + w - 30, y + 1),
        Print("Esc back  A agent  n note")
    )?;
    let mut line = y + 3;
    for text in [
        format!("Status: {}", task.status.label()),
        format!("Priority: {}", task.priority.as_str()),
        format!("Session: {}", task.session.as_deref().unwrap_or("default")),
        format!(
            "Tags: {}",
            if task.tags.is_empty() {
                "none".into()
            } else {
                task.tags
                    .iter()
                    .map(|t| format!("#{t}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        ),
        format!(
            "Files: {}",
            if task.files.is_empty() {
                "none".into()
            } else {
                task.files
                    .iter()
                    .map(|f| format!("@{f}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        ),
    ] {
        queue!(
            stdout,
            MoveTo(x + 2, line),
            Print(trunc(&text, (w - 4) as usize))
        )?;
        line += 1;
    }
    line += 1;
    queue!(
        stdout,
        MoveTo(x + 2, line),
        SetAttribute(Attribute::Bold),
        Print("Agent progress"),
        SetAttribute(Attribute::Reset)
    )?;
    line += 1;
    if task.agents.is_empty() {
        queue!(stdout, MoveTo(x + 4, line), Print("none"))?;
        line += 1;
    } else {
        for a in task.agents.values() {
            if line >= y + h - 5 {
                break;
            }
            let body = a
                .body
                .as_ref()
                .map(|b| format!(" — {}", b))
                .unwrap_or_default();
            queue!(
                stdout,
                MoveTo(x + 4, line),
                Print(trunc(
                    &format!("{}: {}{}", a.agent, a.status.as_str(), body),
                    (w - 6) as usize
                ))
            )?;
            line += 1;
        }
    }
    line += 1;
    queue!(
        stdout,
        MoveTo(x + 2, line),
        SetAttribute(Attribute::Bold),
        Print("Human notes"),
        SetAttribute(Attribute::Reset)
    )?;
    line += 1;
    let notes: Vec<_> = task.notes.iter().filter(|n| n.agent.is_none()).collect();
    if notes.is_empty() {
        queue!(stdout, MoveTo(x + 4, line), Print("none"))?;
    } else {
        for n in notes.iter().rev().take(5).rev() {
            if line >= y + h - 5 {
                break;
            }
            queue!(
                stdout,
                MoveTo(x + 4, line),
                Print(trunc(&n.body, (w - 6) as usize))
            )?;
            line += 1;
        }
    }
    let help = if show_help {
        "Esc back · @ file · A agent · n note · r review · v verify · f fail · c cancel · d done · e edit · ? hide"
    } else {
        "Esc back · @ file · A agent · n note · v verify · f fail · c cancel · q close"
    };
    queue!(
        stdout,
        MoveTo(x + 2, y + h - 2),
        Print(trunc(help, (w - 4) as usize))
    )?;
    stdout.flush()?;
    Ok(())
}

fn ordered_visible(state: &ProjectState) -> Vec<&Task> {
    let mut out = Vec::new();
    for (_, statuses) in sections() {
        for task in state
            .tasks
            .values()
            .filter(|t| statuses.contains(&t.status))
        {
            if task.status.is_active() {
                out.push(task);
            }
        }
    }
    out
}

fn sections() -> Vec<(&'static str, Vec<Status>)> {
    vec![
        ("NOW", vec![Status::Doing]),
        ("REVIEW", vec![Status::AgentDone, Status::NeedsReview]),
        ("FAILED", vec![Status::Failed]),
        ("BLOCKED", vec![Status::Blocked]),
        ("TODO", vec![Status::Todo]),
        ("VERIFIED", vec![Status::Verified]),
        ("DONE", vec![Status::Done]),
        ("CANCELLED", vec![Status::Cancelled]),
    ]
}

fn agent_compact(task: &Task) -> String {
    match task.agents.len() {
        0 => String::new(),
        1 => task.agents.values().next().unwrap().agent.clone(),
        n => format!("{} agents", n),
    }
}

fn visible_session(state: &ProjectState) -> String {
    let mut sessions: Vec<String> = state
        .tasks
        .values()
        .filter_map(|t| t.session.clone())
        .collect();
    sessions.sort();
    sessions.dedup();
    if sessions.len() == 1 {
        sessions.remove(0)
    } else {
        "all sessions".to_string()
    }
}

fn draw_box(stdout: &mut io::Stdout, x: u16, y: u16, w: u16, h: u16) -> Result<()> {
    queue!(
        stdout,
        MoveTo(x, y),
        Print("┌"),
        Print("─".repeat((w - 2) as usize)),
        Print("┐")
    )?;
    for row in 1..h - 1 {
        queue!(
            stdout,
            MoveTo(x, y + row),
            Print("│"),
            MoveTo(x + w - 1, y + row),
            Print("│")
        )?;
    }
    queue!(
        stdout,
        MoveTo(x, y + h - 1),
        Print("└"),
        Print("─".repeat((w - 2) as usize)),
        Print("┘")
    )?;
    Ok(())
}

fn prompt(root: &Path, label: &str, initial: &str) -> Result<String> {
    let mut stdout = io::stdout();
    let mut input = initial.to_string();
    let mut suggestion_idx = 0usize;

    loop {
        let active = files::active_at_query(&input);
        let suggestions = active
            .as_ref()
            .map(|(_, query)| files::search(root, query, 6))
            .unwrap_or_default();
        if suggestion_idx >= suggestions.len() {
            suggestion_idx = suggestions.len().saturating_sub(1);
        }

        let (cols, rows) = terminal::size()?;
        let max_w = cols.saturating_sub(4).max(30);
        let w = max_w.min(88);
        let wanted_h = (suggestions.len() as u16 + 6).clamp(8, 14);
        let max_h = rows.saturating_sub(2).max(8);
        let h = wanted_h.min(max_h);
        let x = (cols.saturating_sub(w)) / 2;
        let y = (rows.saturating_sub(h)) / 2;
        let inner_w = w.saturating_sub(4) as usize;

        clear_rect(&mut stdout, x, y, w, h)?;
        draw_box(&mut stdout, x, y, w, h)?;
        queue!(
            stdout,
            MoveTo(x + 2, y + 1),
            SetAttribute(Attribute::Bold),
            Print(label),
            SetAttribute(Attribute::Reset)
        )?;
        queue!(stdout, MoveTo(x + 2, y + 2), Print(trunc(&input, inner_w)))?;
        queue!(
            stdout,
            MoveTo(x + 2, y + 3),
            Print(trunc(
                "Enter select/save · Tab complete @file · Esc cancel",
                inner_w
            ))
        )?;

        let max_suggestions = h.saturating_sub(6) as usize;
        for (idx, item) in suggestions.iter().take(max_suggestions).enumerate() {
            let line = y + 5 + idx as u16;
            queue!(stdout, MoveTo(x + 2, line))?;
            if idx == suggestion_idx {
                queue!(stdout, SetAttribute(Attribute::Reverse))?;
            }
            queue!(
                stdout,
                Print(format!(
                    "{:<width$}",
                    trunc(&format!("@{}", item.path), inner_w),
                    width = inner_w
                ))
            )?;
            if idx == suggestion_idx {
                queue!(stdout, SetAttribute(Attribute::Reset))?;
            }
        }
        stdout.flush()?;

        if let CtEvent::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc => return Ok(String::new()),
                KeyCode::Enter => {
                    if let (Some((start, _)), Some(item)) =
                        (active, suggestions.get(suggestion_idx))
                    {
                        input = files::complete_at(&input, start, &item.path);
                    }
                    return Ok(input.trim_end().to_string());
                }
                KeyCode::Tab => {
                    if let (Some((start, _)), Some(item)) =
                        (active, suggestions.get(suggestion_idx))
                    {
                        input = files::complete_at(&input, start, &item.path);
                    }
                }
                KeyCode::Up => suggestion_idx = suggestion_idx.saturating_sub(1),
                KeyCode::Down => {
                    if suggestion_idx + 1 < suggestions.len() {
                        suggestion_idx += 1;
                    }
                }
                KeyCode::Backspace => {
                    input.pop();
                    suggestion_idx = 0;
                }
                KeyCode::Char(ch) => {
                    input.push(ch);
                    suggestion_idx = 0;
                }
                _ => {}
            }
        }
    }
}

fn clear_rect(stdout: &mut io::Stdout, x: u16, y: u16, w: u16, h: u16) -> Result<()> {
    let blank = " ".repeat(w as usize);
    for row in 0..h {
        queue!(stdout, MoveTo(x, y + row), Print(&blank))?;
    }
    Ok(())
}

fn trunc(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.into()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}
