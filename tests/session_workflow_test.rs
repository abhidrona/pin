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
fn positive_full_session_workflow_across_fscrm_and_pomo() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");
    let pomo = project("pomo");

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["a", "Fix Recover filters", "-p", "high"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added task #1: Fix Recover filters"));

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["add", "Validate backend filter support"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added task #2: Validate backend filter support"));

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["add", "Run browser smoke test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added task #3: Run browser smoke test"));

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["start", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started task #1"));

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["note", "1", "--agent", "claude", "Changed Recover page URL params"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added note to task #1"));

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["ad", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Marked review task #1"));

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["review", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Marked needs-review task #1"));

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["fail", "1", "failed filter returns empty data"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Marked failed task #1"));

    cmd(&home)
        .current_dir(fscrm.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Project:"))
        .stdout(predicate::str::contains("[1] failed"))
        .stdout(predicate::str::contains("high"))
        .stdout(predicate::str::contains("Fix Recover filters"))
        .stdout(predicate::str::contains("[2] todo"))
        .stdout(predicate::str::contains("Validate backend filter support"))
        .stdout(predicate::str::contains("[3] todo"))
        .stdout(predicate::str::contains("Run browser smoke test"));

    cmd(&home)
        .current_dir(fscrm.path())
        .arg("show")
        .assert()
        .success()
        .stdout(predicate::str::contains("FAILED"))
        .stdout(predicate::str::contains("TODO"))
        .stdout(predicate::str::contains("NEXT"));

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Agent progress"))
        .stdout(predicate::str::contains("failed filter returns empty data"));

    cmd(&home)
        .current_dir(fscrm.path())
        .arg("brief")
        .assert()
        .success()
        .stdout(predicate::str::contains("Focus:"))
        .stdout(predicate::str::contains("#1 Fix Recover filters"))
        .stdout(predicate::str::contains("Recent notes:"))
        .stdout(predicate::str::contains("failed filter returns empty data"));

    // In tests stdout is not a terminal, so overlay must not launch an
    // interactive TUI. It should render the same plain project HUD as show.
    cmd(&home)
        .current_dir(fscrm.path())
        .arg("overlay")
        .assert()
        .success()
        .stdout(predicate::str::contains("FAILED"))
        .stdout(predicate::str::contains("Fix Recover filters"));

    cmd(&home)
        .current_dir(pomo.path())
        .args(["add", "Fix notification newline escaping"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added task #1: Fix notification newline escaping"));

    cmd(&home)
        .current_dir(pomo.path())
        .args(["review", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Marked needs-review task #1"));

    cmd(&home)
        .current_dir(fscrm.path())
        .arg("all")
        .assert()
        .success()
        .stdout(predicate::str::contains("All active tasks"))
        .stdout(predicate::str::contains("Fix Recover filters"))
        .stdout(predicate::str::contains("Validate backend filter support"))
        .stdout(predicate::str::contains("Run browser smoke test"))
        .stdout(predicate::str::contains("Fix notification newline escaping"));

    let events = read_log_lines(&fscrm);
    let ops: Vec<_> = events.iter().map(|ev| ev["op"].as_str().unwrap()).collect();
    assert_eq!(
        ops,
        vec![
            "task.add",
            "task.add",
            "task.add",
            "task.status",
            "task.note",
            "task.status",
            "task.status",
            "task.status",
        ]
    );
    assert_eq!(events[0]["id"], 1);
    assert_eq!(events[0]["priority"], "high");
    assert_eq!(events[3]["status"], "doing");
    assert_eq!(events[4]["agent"], "claude");
    assert_eq!(events[5]["status"], "needs-review");
    assert_eq!(events[6]["status"], "needs-review");
    assert_eq!(events[7]["status"], "failed");
    assert_eq!(events[7]["body"], "failed filter returns empty data");
}

#[test]
fn positive_all_status_filter_finds_needs_review_across_projects() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");
    let pomo = project("pomo");

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["add", "Fix Recover filters"])
        .assert()
        .success();
    cmd(&home)
        .current_dir(fscrm.path())
        .args(["fail", "1", "failed filter returns empty data"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(pomo.path())
        .args(["add", "Fix notification newline escaping"])
        .assert()
        .success();
    cmd(&home)
        .current_dir(pomo.path())
        .args(["review", "1"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["all", "--status", "needs-review"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Fix notification newline escaping"))
        .stdout(predicate::str::contains("Fix Recover filters").not());
}

#[test]
fn positive_agent_report_records_agent_note_and_moves_task_to_review() {
    let home = TempDir::new().unwrap();
    let fscrm = project("fscrm");

    cmd(&home)
        .current_dir(fscrm.path())
        .args(["add", "Fix Recover filters"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(fscrm.path())
        .args([
            "report",
            "1",
            "--agent",
            "claude",
            "Changed Recover page URL params",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Recorded claude report for task #1"))
        .stdout(predicate::str::contains("Human verification required"));

    cmd(&home)
        .current_dir(fscrm.path())
        .arg("brief")
        .assert()
        .success()
        .stdout(predicate::str::contains("Focus"))
        .stdout(predicate::str::contains("Status: review"))
        .stdout(predicate::str::contains("claude"))
        .stdout(predicate::str::contains("Changed Recover page URL params"));

    let events = read_log_lines(&fscrm);
    assert_eq!(events[0]["op"], "task.add");
    assert_eq!(events[1]["op"], "agent.progress");
    assert_eq!(events[1]["agent"], "claude");
    assert_eq!(events[1]["agent_status"], "reported");
    assert_eq!(events[1]["body"], "Changed Recover page URL params");
    assert_eq!(events[2]["op"], "task.status");
    assert_eq!(events[2]["status"], "needs-review");
}
