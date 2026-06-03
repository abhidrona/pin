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

fn project(prefix: &str) -> TempDir { Builder::new().prefix(prefix).tempdir().unwrap() }

fn read_log_lines(project: &TempDir) -> Vec<Value> {
    let log_path = project.path().join(".pin/log.jsonl");
    let data = fs::read_to_string(&log_path).unwrap();
    data.lines().filter(|line| !line.trim().is_empty()).map(|line| serde_json::from_str::<Value>(line).unwrap()).collect()
}

#[test]
fn agent_progress_is_separate_from_task_status_and_visible_in_detail() {
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
        .success();

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["-s", "recover-filters", "ag", "1", "claude", "working", "Changing Recover page URL params"])
        .assert()
        .success()
        .stdout(predicate::str::contains("working"));

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["-s", "recover-filters", "show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Status: doing"))
        .stdout(predicate::str::contains("Agent progress"))
        .stdout(predicate::str::contains("claude: working"))
        .stdout(predicate::str::contains("Changing Recover page URL params"));

    let events = read_log_lines(&fscrm);
    assert_eq!(events[2]["op"], "agent.progress");
    assert_eq!(events[2]["agent"], "claude");
    assert_eq!(events[2]["agent_status"], "working");
}

#[test]
fn agent_report_moves_task_to_review_but_does_not_mark_done() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");

    cmd(&home).current_dir(fscrm.path()).args(["a", "Validate backend filter support"]).assert().success();

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["report", "1", "-a", "codex", "Checked repository filters"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Task moved to review"));

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Status: review"))
        .stdout(predicate::str::contains("codex: reported"))
        .stdout(predicate::str::contains("Checked repository filters"))
        .stdout(predicate::str::contains("Status: done").not());

    let events = read_log_lines(&fscrm);
    assert_eq!(events[1]["op"], "agent.progress");
    assert_eq!(events[1]["agent_status"], "reported");
    assert_eq!(events[2]["op"], "task.status");
    assert_eq!(events[2]["status"], "needs-review");
}

#[test]
fn sessions_command_summarizes_multiple_workstreams_in_same_folder() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");

    cmd(&home).current_dir(fscrm.path()).args(["-s", "recover-filters", "a", "Fix failed filter"]).assert().success();
    cmd(&home).current_dir(fscrm.path()).args(["-s", "recover-filters", "s", "1"]).assert().success();
    cmd(&home).current_dir(fscrm.path()).args(["-s", "quote-room", "a", "Verify quote room flow"]).assert().success();
    cmd(&home).current_dir(fscrm.path()).args(["-s", "quote-room", "r", "2"]).assert().success();

    cmd(&home)
        .current_dir(fscrm.path())
        .arg("ss")
        .assert()
        .success()
        .stdout(predicate::str::contains("ACTIVE SESSIONS"))
        .stdout(predicate::str::contains("recover-filters"))
        .stdout(predicate::str::contains("1 now"))
        .stdout(predicate::str::contains("quote-room"))
        .stdout(predicate::str::contains("1 review"));
}

#[test]
fn mark_ls_is_session_scoped_and_simple() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");

    cmd(&home).current_dir(fscrm.path()).args(["-s", "recover-filters", "a", "Fix failed filter"]).assert().success();
    cmd(&home).current_dir(fscrm.path()).args(["-s", "quote-room", "a", "Verify quote room flow"]).assert().success();

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["-s", "recover-filters", "ls"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Fix failed filter"))
        .stdout(predicate::str::contains("Verify quote room flow").not());
}

#[test]
fn negative_agent_progress_rejects_blank_body_and_does_not_append() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");

    cmd(&home).current_dir(fscrm.path()).args(["a", "Fix Recover filters"]).assert().success();
    let before = read_log_lines(&fscrm).len();

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["ag", "1", "claude", "working", "   "])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Agent progress note cannot be empty"));

    assert_eq!(read_log_lines(&fscrm).len(), before);
}
