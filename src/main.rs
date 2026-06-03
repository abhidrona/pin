mod model;
mod ops;
mod project;
mod registry;
mod render;
mod storage;
mod tui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use model::{AgentStatus, Priority, Status};
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "pin", version, about = "Directory-aware terminal task tracker")]
struct Cli {
    #[arg(long, global = true)]
    root: Option<PathBuf>,

    /// Scope commands to a client/session, for example claude, codex, human.
    /// If omitted, PIN_SESSION is used when set.
    #[arg(short = 's', long, global = true)]
    session: Option<String>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Init,
    #[command(alias = "a")]
    Add {
        title: String,
        #[arg(short = 'p', long, value_enum, default_value_t = Priority::Normal)]
        priority: Priority,
        #[arg(long)]
        agent: Option<String>,
        /// Add one or more tags. Can be repeated or comma-separated.
        #[arg(short = 't', long = "tag")]
        tags: Vec<String>,
    },
    #[command(alias = "ls")]
    List {
        #[arg(long, value_enum)]
        status: Option<Status>,
        #[arg(long)]
        active: bool,
        /// Filter by tag. Can be repeated or comma-separated.
        #[arg(short = 't', long = "tag")]
        tags: Vec<String>,
    },
    All {
        #[arg(long)]
        include_done: bool,
        #[arg(long, value_enum)]
        status: Option<Status>,
        #[arg(short = 't', long = "tag")]
        tags: Vec<String>,
    },
    #[command(alias = "s")]
    Start { id: u64 },
    #[command(alias = "ad", alias = "claim", alias = "claimed", alias = "reported")]
    AgentDone { id: u64 },
    #[command(alias = "r")]
    Review { id: u64 },
    #[command(alias = "ok")]
    Verified { id: u64, body: Option<String> },
    #[command(alias = "f")]
    Fail { id: u64, body: String },
    #[command(alias = "b")]
    Block { id: u64, body: String },
    #[command(alias = "c", alias = "cancelled")]
    Cancel { id: u64 },
    #[command(alias = "d")]
    Done { id: u64 },
    #[command(alias = "n")]
    Note { id: u64, body: String, #[arg(long)] agent: Option<String> },
    /// Add tags to a task. Tags can be repeated or comma-separated.
    Tag { id: u64, tags: Vec<String> },
    /// Remove tags from a task. Tags can be repeated or comma-separated.
    Untag { id: u64, tags: Vec<String> },
    /// Record agent progress without changing human task truth.
    #[command(alias = "ag", alias = "progress")]
    Agent { id: u64, agent: String, #[arg(value_enum)] status: AgentStatus, body: String },
    /// Record a human-approved agent summary and move the task to review.
    #[command(alias = "report", alias = "ar")]
    AgentReport { id: u64, #[arg(long, short = 'a')] agent: String, body: String },
    Edit { id: u64, title: String },
    #[command(alias = "view")]
    Show {
        id: Option<u64>,
        #[arg(short = 't', long = "tag")]
        tags: Vec<String>,
        /// Show all sessions in the folder instead of only the active session.
        #[arg(long)]
        all_sessions: bool,
    },
    #[command(alias = "br")]
    Brief {
        #[arg(short = 't', long = "tag")]
        tags: Vec<String>,
        #[arg(long)]
        all_sessions: bool,
    },
    /// List active sessions in the current folder/worktree.
    #[command(alias = "ss")]
    Sessions,
    #[command(alias = "o")]
    Overlay {
        #[arg(short = 't', long = "tag")]
        tags: Vec<String>,
    },
    #[command(alias = "ui")]
    Tui {
        #[arg(short = 't', long = "tag")]
        tags: Vec<String>,
    },
    TmuxInstall { #[arg(long, default_value = "T")] key: String },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let root = project::detect_project_root(cli.root)?;
    let session = effective_session(cli.session);

    match cli.cmd {
        Cmd::Init => {
            let created = storage::init_project(&root)?;
            register_best_effort(&root);
            if created { println!("Initialized pin in {}", root.display()); }
            else { println!("pin already initialized in {}", root.display()); }
        }
        Cmd::Add { title, priority, agent, tags } => {
            let created = storage::init_project(&root)?;
            register_best_effort(&root);
            if created { println!("Initialized pin in {}", root.display()); }
            let tags = ops::normalize_tags(tags);
            let id = ops::add(&root, title.clone(), priority, agent, tags, Some(session.clone().unwrap_or_else(|| "default".to_string())))?;
            println!("Added task #{}: {}", id, title);
        }
        Cmd::List { status, active, tags } => {
            let state = filtered_state(&root, session.as_deref().or(Some("default")), tags)?;
            println!("{}", render::list(&state, status, active));
        }
        Cmd::All { include_done, status, tags } => all(include_done, status, session.as_deref(), ops::normalize_tags(tags))?,
        Cmd::Start { id } => set_status(&root, id, Status::Doing, None, "Started")?,
        Cmd::AgentDone { id } => set_status(&root, id, Status::NeedsReview, None, "Marked review")?,
        Cmd::Review { id } => set_status(&root, id, Status::NeedsReview, None, "Marked needs-review")?,
        Cmd::Verified { id, body } => set_status(&root, id, Status::Verified, body, "Verified")?,
        Cmd::Fail { id, body } => set_status(&root, id, Status::Failed, Some(body), "Marked failed")?,
        Cmd::Block { id, body } => set_status(&root, id, Status::Blocked, Some(body), "Blocked")?,
        Cmd::Cancel { id } => set_status(&root, id, Status::Cancelled, None, "Cancelled")?,
        Cmd::Done { id } => set_status(&root, id, Status::Done, None, "Done")?,
        Cmd::Note { id, body, agent } => { ops::note(&root, id, body, agent)?; println!("Added note to task #{}.", id); }
        Cmd::Tag { id, tags } => { ops::add_tags(&root, id, tags)?; println!("Tagged task #{}.", id); }
        Cmd::Untag { id, tags } => { ops::remove_tags(&root, id, tags)?; println!("Removed tags from task #{}.", id); }
        Cmd::Agent { id, agent, status, body } => {
            let agent_name = agent.clone();
            ops::agent_progress(&root, id, agent, status, body)?;
            println!("Recorded {agent_name} progress for task #{id}: {}.", status.as_str());
        }
        Cmd::AgentReport { id, agent, body } => {
            let agent_name = agent.clone();
            ops::agent_report(&root, id, agent, body)?;
            println!("Recorded {agent_name} report for task #{id}. Task moved to review. Human verification required.");
        }
        Cmd::Edit { id, title } => { ops::set_title(&root, id, title)?; println!("Edited task #{}.", id); }
        Cmd::Show { id, tags, all_sessions } => {
            let state = filtered_state(&root, if all_sessions { None } else { session.as_deref().or(Some("default")) }, tags)?;
            if let Some(id) = id { println!("{}", render::task_detail(&state, id)); }
            else { println!("{}", render::show(&state)); }
        }
        Cmd::Brief { tags, all_sessions } => { let state = filtered_state(&root, if all_sessions { None } else { session.as_deref().or(Some("default")) }, tags)?; println!("{}", render::brief(&state)); }
        Cmd::Sessions => { let state = storage::load_state(&root)?; println!("{}", render::sessions(&state)); }
        Cmd::Overlay { tags } => overlay(&root, session.as_deref(), ops::normalize_tags(tags))?,
        Cmd::Tui { tags } => tui::run(root, session.or_else(|| Some("default".to_string())), ops::normalize_tags(tags))?,
        Cmd::TmuxInstall { key } => {
            println!("bind-key {} display-popup -w 80% -h 70% -E \"pin ui\"", key);
        }
    }

    Ok(())
}

fn effective_session(cli_session: Option<String>) -> Option<String> {
    cli_session
        .or_else(|| std::env::var("PIN_SESSION").ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn filtered_state(root: &std::path::Path, session: Option<&str>, tags: Vec<String>) -> Result<model::ProjectState> {
    let state = storage::load_state(root)?;
    Ok(state.filtered(session, &ops::normalize_tags(tags)))
}

fn set_status(root: &std::path::Path, id: u64, status: Status, body: Option<String>, label: &str) -> Result<()> {
    ops::status(root, id, status, body)?;
    println!("{} task #{}.", label, id);
    Ok(())
}

fn all(include_done: bool, status: Option<Status>, session: Option<&str>, tags: Vec<String>) -> Result<()> {
    let projects = registry::load_projects()?;
    println!("All {}tasks{}{}\n",
        if include_done { "" } else { "active " },
        session.map(|s| format!(" for session '{s}'")).unwrap_or_default(),
        if tags.is_empty() { String::new() } else { format!(" tagged {}", tags.iter().map(|t| format!("#{t}")).collect::<Vec<_>>().join(", ")) }
    );
    for p in projects {
        let state = match storage::load_state(&p.root) {
            Ok(s) => s.filtered(session, &tags),
            Err(e) => { eprintln!("warning: skipping {}: {}", p.root.display(), e); continue; }
        };
        let mut lines = Vec::new();
        for task in state.tasks.values() {
            if !include_done && !task.status.is_active() { continue; }
            if let Some(s) = status { if !status_matches(task.status, s) { continue; } }
            lines.push(render::task_line(task));
        }
        if lines.is_empty() { continue; }
        println!("{}  {}", state.meta.name, state.meta.root.display());
        for line in lines { println!("  {}", line); }
        println!();
    }
    Ok(())
}

fn overlay(root: &std::path::Path, session: Option<&str>, tags: Vec<String>) -> Result<()> {
    let multi_session = if session.is_none() {
        let state = storage::load_state(root)?;
        render::session_summaries(&state).len() > 1
    } else { false };

    if std::env::var_os("TMUX").is_some() {
        let mut command = String::from("pin");
        if multi_session {
            command.push_str(" sessions");
        } else {
            let effective = session.unwrap_or("default");
            command.push_str(&format!(" --session {} tui", shell_escape(effective)));
            for tag in &tags { command.push_str(&format!(" --tag {}", shell_escape(tag))); }
        }
        let status = Command::new("tmux")
            .arg("display-popup")
            .arg("-w").arg("80%")
            .arg("-h").arg("70%")
            .arg("-E")
            .arg(command)
            .status();
        match status {
            Ok(s) if s.success() => return Ok(()),
            _ => eprintln!("warning: failed to open tmux popup. falling back to plain output."),
        }
    }

    if !std::io::stdout().is_terminal() || multi_session {
        let state = storage::load_state(root)?;
        if multi_session {
            println!("{}", render::sessions(&state));
        } else {
            let effective = session.or(Some("default"));
            println!("{}", render::show(&state.filtered(effective, &tags)));
        }
        return Ok(());
    }

    tui::run(root.to_path_buf(), Some(session.unwrap_or("default").to_string()), tags)
}

fn register_best_effort(root: &std::path::Path) {
    let name = project::project_name(root);
    if let Err(err) = registry::register_project(&name, root) {
        eprintln!("warning: failed to update global registry: {err:#}");
    }
}

fn status_matches(actual: Status, wanted: Status) -> bool {
    actual == wanted || (wanted == Status::AgentDone && actual == Status::NeedsReview)
}

fn shell_escape(s: &str) -> String {
    if s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '/') {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}
