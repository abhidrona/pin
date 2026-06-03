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
fn negative_cannot_note_or_change_missing_task_in_workflow() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["add", "Fix Recover filters"])
        .assert()
        .success();

    let before = read_log_lines(&fscrm).len();

    cmd(&home)
        .current_dir(fscrm.path())
        .args([
            "note",
            "99",
            "--agent",
            "claude",
            "Changed Recover page URL params",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Task #99 not found"));

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["review", "99"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Task #99 not found"));

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["fail", "99", "failed filter returns empty data"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Task #99 not found"));

    assert_eq!(
        read_log_lines(&fscrm).len(),
        before,
        "failed commands must not append events"
    );
}

#[test]
fn negative_fail_requires_non_empty_reason_and_does_not_append() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["add", "Fix Recover filters"])
        .assert()
        .success();

    let before = read_log_lines(&fscrm).len();

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["fail", "1", "   "])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed reason is required"));

    assert_eq!(
        read_log_lines(&fscrm).len(),
        before,
        "blank fail reason must not append task.status"
    );
}

#[test]
fn negative_invalid_status_filter_is_rejected_by_cli() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["add", "Fix Recover filters"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["all", "--status", "not-real"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn negative_overlay_on_uninitialized_project_fails_cleanly_in_non_tty() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");

    cmd(&home)
        .current_dir(fscrm.path())
        .arg("overlay")
        .assert()
        .failure()
        .stderr(predicate::str::contains("pin is not initialized"));
}

#[test]
fn negative_all_skips_deleted_registered_project_and_still_shows_existing_project() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");
    let deleted = project("deleted");

    cmd(&home)
        .current_dir(deleted.path())
        .args(["add", "Task from deleted project"])
        .assert()
        .success();
    let deleted_root = deleted.path().to_path_buf();
    drop(deleted);
    assert!(
        !deleted_root.exists(),
        "TempDir drop should delete the registered project"
    );

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["add", "Fix Recover filters"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(fscrm.path())
        .arg("all")
        .assert()
        .success()
        .stderr(predicate::str::contains("warning: skipping"))
        .stdout(predicate::str::contains("Fix Recover filters"))
        .stdout(predicate::str::contains("Task from deleted project").not());
}

#[test]
fn negative_corrupt_log_causes_show_and_brief_to_fail_without_overwriting_log() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["add", "Fix Recover filters"])
        .assert()
        .success();

    let log_path = fscrm.path().join(".pin/log.jsonl");
    fs::write(&log_path, "this is not json\n").unwrap();

    cmd(&home)
        .current_dir(fscrm.path())
        .arg("show")
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to parse"));

    cmd(&home)
        .current_dir(fscrm.path())
        .arg("brief")
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to parse"));

    let data = fs::read_to_string(log_path).unwrap();
    assert_eq!(
        data, "this is not json\n",
        "read failures must not overwrite corrupt logs"
    );
}

#[test]
fn negative_agent_report_requires_existing_task_agent_and_body() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["add", "Fix Recover filters"])
        .assert()
        .success();

    let before = read_log_lines(&fscrm).len();

    cmd(&home)
        .current_dir(fscrm.path())
        .args([
            "agent-report",
            "99",
            "--agent",
            "claude",
            "Changed URL params",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Task #99 not found"));

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["agent-report", "1", "--agent", "   ", "Changed URL params"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Agent name cannot be empty"));

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["agent-report", "1", "--agent", "claude", "   "])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Agent report cannot be empty"));

    assert_eq!(
        read_log_lines(&fscrm).len(),
        before,
        "invalid agent reports must not append events"
    );
}
