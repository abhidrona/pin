use assert_cmd::prelude::*;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::{Builder, TempDir};

fn cmd(home: &TempDir) -> Command {
    let mut c = Command::cargo_bin("pin").expect("binary pin should build");
    c.env("XDG_DATA_HOME", home.path().join("xdg-data"));
    c.env("HOME", home.path());
    c
}

fn project(prefix: &str) -> TempDir {
    Builder::new().prefix(prefix).tempdir().unwrap()
}

fn read_log_lines(project: &TempDir) -> Vec<Value> {
    let log_path = project.path().join(".pin/log.jsonl");
    let data = fs::read_to_string(&log_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", log_path.display()));
    data.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<Value>(line).expect("valid jsonl event"))
        .collect()
}

#[test]
fn tags_are_stored_displayed_and_filterable() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["add", "Fix Recover filters", "-p", "high", "-t", "ui", "-t", "agent,backend"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["add", "Update README", "-t", "docs"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["list", "-t", "ui"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Fix Recover filters"))
        .stdout(predicate::str::contains("#ui"))
        .stdout(predicate::str::contains("#backend"))
        .stdout(predicate::str::contains("Update README").not());

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["tag", "2", "human", "non-agent"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["show", "-t", "non-agent"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Update README"))
        .stdout(predicate::str::contains("#non-agent"))
        .stdout(predicate::str::contains("Fix Recover filters").not());

    let events = read_log_lines(&fscrm);
    assert_eq!(events[0]["tags"], serde_json::json!(["agent", "backend", "ui"]));
    assert_eq!(events[2]["op"], "task.tags.add");
}

#[test]
fn sessions_allow_claude_and_codex_to_share_one_folder_without_mixing_tasks() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["-s", "claude", "add", "Wire Recover filters", "-t", "ui"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["-s", "codex", "add", "Validate backend filter support", "-t", "backend"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["-s", "claude", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Session: claude"))
        .stdout(predicate::str::contains("Wire Recover filters"))
        .stdout(predicate::str::contains("Validate backend filter support").not());

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["-s", "codex", "brief"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Session: codex"))
        .stdout(predicate::str::contains("Validate backend filter support"))
        .stdout(predicate::str::contains("Wire Recover filters").not());

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["all", "-t", "backend"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Validate backend filter support"))
        .stdout(predicate::str::contains("Wire Recover filters").not());

    let events = read_log_lines(&fscrm);
    assert_eq!(events[0]["session"], "claude");
    assert_eq!(events[1]["session"], "codex");
}

#[test]
fn mark_session_env_scopes_add_and_list() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");

    cmd(&home)
        .current_dir(fscrm.path())
        .env("PIN_SESSION", "claude")
        .args(["add", "Task owned by claude"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(fscrm.path())
        .env("PIN_SESSION", "codex")
        .args(["add", "Task owned by codex"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(fscrm.path())
        .env("PIN_SESSION", "claude")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Task owned by claude"))
        .stdout(predicate::str::contains("Task owned by codex").not());
}

#[test]
fn git_worktree_marker_file_is_treated_as_project_root() {
    let home = TempDir::new().unwrap();
    let worktree = project("fscrm-worktree");
    let nested = worktree.path().join("backend/app/services");
    fs::create_dir_all(&nested).unwrap();
    fs::write(worktree.path().join(".git"), "gitdir: /tmp/main/.git/worktrees/fscrm-worktree\n").unwrap();

    cmd(&home)
        .current_dir(&nested)
        .args(["add", "Fix service layer in worktree"])
        .assert()
        .success();

    assert!(worktree.path().join(".pin/log.jsonl").exists());
    assert!(!nested.join(".pin/log.jsonl").exists());

    cmd(&home)
        .current_dir(&nested)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Fix service layer in worktree"));
}

#[test]
fn negative_blank_tag_is_rejected_for_tag_command() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["add", "Fix Recover filters"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["tag", "1", "   "])
        .assert()
        .failure()
        .stderr(predicate::str::contains("At least one tag is required"));
}

#[test]
fn negative_session_filter_can_return_empty_without_crashing() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["-s", "claude", "add", "Claude task"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["-s", "codex", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Claude task").not());
}

#[test]
fn short_flags_and_aliases_work_for_daily_flow() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["-s", "recover-filters", "a", "Fix Recover filters", "-p", "high", "-t", "ui"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["-s", "recover-filters", "s", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started task #1"));

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["-s", "recover-filters", "report", "1", "--agent", "claude", "Changed URL params"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Task moved to review"));

    // The overview/list stays intentionally simple: it shows task truth, not full agent progress.
    cmd(&home)
        .current_dir(fscrm.path())
        .args(["-s", "recover-filters", "view"])
        .assert()
        .success()
        .stdout(predicate::str::contains("REVIEW"))
        .stdout(predicate::str::contains("Fix Recover filters"));

    // Agent progress is visible in task details.
    cmd(&home)
        .current_dir(fscrm.path())
        .args(["-s", "recover-filters", "show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("claude: reported"))
        .stdout(predicate::str::contains("Changed URL params"));

    // `reported` remains a convenient status alias for tasks moved to review by agent reports.
    cmd(&home)
        .current_dir(fscrm.path())
        .args(["all", "--status", "reported"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Fix Recover filters"));
}
