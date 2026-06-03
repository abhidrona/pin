use assert_cmd::prelude::*;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn cmd(home: &TempDir) -> Command {
    let mut c = Command::cargo_bin("pin").expect("binary pin should build");
    c.env("XDG_DATA_HOME", home.path().join("xdg-data"));
    c.env("HOME", home.path());
    c
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
fn human_can_mark_task_done_or_cancelled_and_active_list_excludes_both() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();

    cmd(&home)
        .current_dir(project.path())
        .args(["a", "Ship login redirect fix"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(project.path())
        .args(["a", "Remove obsolete auth experiment"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(project.path())
        .args(["done", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Done task #1"));

    cmd(&home)
        .current_dir(project.path())
        .args(["cancel", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cancelled task #2"));

    cmd(&home)
        .current_dir(project.path())
        .args(["ls", "--active"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Ship login redirect fix").not())
        .stdout(predicate::str::contains("Remove obsolete auth experiment").not());

    cmd(&home)
        .current_dir(project.path())
        .args(["ls", "--status", "cancelled"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[2] cancelled"))
        .stdout(predicate::str::contains("Remove obsolete auth experiment"));

    let events = read_log_lines(&project);
    assert_eq!(events.iter().filter(|e| e["op"] == "task.status" && e["status"] == "done").count(), 1);
    assert_eq!(events.iter().filter(|e| e["op"] == "task.status" && e["status"] == "cancelled").count(), 1);
}

#[test]
fn cancel_alias_c_closes_task_without_completion() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();

    cmd(&home)
        .current_dir(project.path())
        .args(["a", "Obsolete token experiment"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(project.path())
        .args(["c", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cancelled task #1"));

    cmd(&home)
        .current_dir(project.path())
        .args(["show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Status: cancelled"));
}

#[test]
fn cancel_missing_task_fails_and_does_not_append() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();

    cmd(&home)
        .current_dir(project.path())
        .args(["a", "Fix auth smoke test"])
        .assert()
        .success();

    let before = read_log_lines(&project).len();

    cmd(&home)
        .current_dir(project.path())
        .args(["cancel", "99"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Task #99 not found"));

    assert_eq!(read_log_lines(&project).len(), before);
}
