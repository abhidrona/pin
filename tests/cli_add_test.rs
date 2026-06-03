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
fn add_from_terminal_auto_initializes_project_and_appends_jsonl_event() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();

    cmd(&home)
        .current_dir(project.path())
        .args(["add", "Fix terminal add", "-p", "high", "--agent", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized pin"))
        .stdout(predicate::str::contains("Added task #1: Fix terminal add"));

    assert!(project.path().join(".pin/meta.json").exists());
    assert!(project.path().join(".pin/log.jsonl").exists());

    let events = read_log_lines(&project);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["op"], "task.add");
    assert_eq!(events[0]["id"], 1);
    assert_eq!(events[0]["title"], "Fix terminal add");
    assert_eq!(events[0]["priority"], "high");
    assert_eq!(events[0]["agent"], "claude");
}

#[test]
fn add_multiple_tasks_from_terminal_assigns_incrementing_ids() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();

    cmd(&home)
        .current_dir(project.path())
        .args(["add", "First task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added task #1"));

    cmd(&home)
        .current_dir(project.path())
        .args(["add", "Second task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added task #2"));

    cmd(&home)
        .current_dir(project.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("[1] todo"))
        .stdout(predicate::str::contains("First task"))
        .stdout(predicate::str::contains("[2] todo"))
        .stdout(predicate::str::contains("Second task"));

    let events = read_log_lines(&project);
    assert_eq!(events[0]["id"], 1);
    assert_eq!(events[1]["id"], 2);
}

#[test]
fn add_empty_title_from_terminal_fails_without_writing_event() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();

    cmd(&home)
        .current_dir(project.path())
        .args(["add", "   "])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Task title cannot be empty"));

    let log_path = project.path().join(".pin/log.jsonl");
    assert!(
        log_path.exists(),
        "add auto-initializes before validating title"
    );
    let data = fs::read_to_string(log_path).unwrap();
    assert!(
        data.trim().is_empty(),
        "invalid add must not append an event"
    );
}

#[test]
fn add_from_nested_directory_uses_git_root() {
    let home = TempDir::new().unwrap();
    let repo = TempDir::new().unwrap();
    fs::create_dir(repo.path().join(".git")).unwrap();
    let nested = repo.path().join("backend/app/services");
    fs::create_dir_all(&nested).unwrap();

    cmd(&home)
        .current_dir(&nested)
        .args(["add", "Fix service layer"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added task #1"));

    assert!(repo.path().join(".pin/log.jsonl").exists());
    assert!(!nested.join(".pin/log.jsonl").exists());
}

#[test]
fn list_status_filter_works_for_terminal_added_task() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();

    cmd(&home)
        .current_dir(project.path())
        .args(["add", "Run smoke test"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(project.path())
        .args(["start", "1"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(project.path())
        .args(["list", "--status", "doing"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run smoke test"))
        .stdout(predicate::str::contains("doing"));

    cmd(&home)
        .current_dir(project.path())
        .args(["list", "--status", "blocked"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run smoke test").not());
}

#[test]
fn all_shows_tasks_added_from_terminal_across_registered_folders() {
    let home = TempDir::new().unwrap();
    let fscrm = TempDir::new().unwrap();
    let pomo = TempDir::new().unwrap();

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["add", "Fix Recover filters"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(pomo.path())
        .args(["add", "Fix notification escaping"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(fscrm.path())
        .arg("all")
        .assert()
        .success()
        .stdout(predicate::str::contains("Fix Recover filters"))
        .stdout(predicate::str::contains("Fix notification escaping"));
}
