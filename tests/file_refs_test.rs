use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn bin() -> Command {
    Command::cargo_bin("pin").unwrap()
}

fn setup() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("src/auth")).unwrap();
    fs::create_dir_all(dir.path().join("tests/auth")).unwrap();
    fs::write(dir.path().join("src/auth/login.rs"), "// login").unwrap();
    fs::write(dir.path().join("src/auth/token_refresh.rs"), "// token").unwrap();
    fs::write(dir.path().join("tests/auth/login_smoke_test.rs"), "// test").unwrap();
    dir
}

fn read_events(root: &Path) -> Vec<Value> {
    fs::read_to_string(root.join(".pin/log.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn files_command_fuzzy_searches_paths() {
    let project = setup();

    bin()
        .current_dir(project.path())
        .args(["files", "lgnsmk"])
        .assert()
        .success()
        .stdout(predicate::str::contains("@tests/auth/login_smoke_test.rs"));
}

#[test]
fn add_extracts_fuzzy_at_file_references() {
    let project = setup();

    bin()
        .current_dir(project.path())
        .args(["a", "Fix redirect in @lgn", "-t", "auth"])
        .assert()
        .success();

    bin()
        .current_dir(project.path())
        .args(["show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Fix redirect in @src/auth/login.rs",
        ))
        .stdout(predicate::str::contains("Files:"))
        .stdout(predicate::str::contains("@src/auth/login.rs"));
}

#[test]
fn explicit_file_attach_and_unattach_work() {
    let project = setup();

    bin()
        .current_dir(project.path())
        .args(["a", "Validate token refresh", "-F", "tokref"])
        .assert()
        .success();

    bin()
        .current_dir(project.path())
        .args(["show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("@src/auth/token_refresh.rs"));

    bin()
        .current_dir(project.path())
        .args(["unref", "1", "tokref"])
        .assert()
        .success();

    bin()
        .current_dir(project.path())
        .args(["show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Files:").not());
}

#[test]
fn notes_failures_and_agent_progress_attach_refs() {
    let project = setup();

    bin()
        .current_dir(project.path())
        .args(["a", "Fix auth"])
        .assert()
        .success();
    bin()
        .current_dir(project.path())
        .args(["n", "1", "Repro in @lgnsmk"])
        .assert()
        .success();
    bin()
        .current_dir(project.path())
        .args(["ag", "1", "claude", "working", "Checking @tokref"])
        .assert()
        .success();
    bin()
        .current_dir(project.path())
        .args(["f", "1", "Still failing in @lgnsmk"])
        .assert()
        .success();

    bin()
        .current_dir(project.path())
        .args(["show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("@tests/auth/login_smoke_test.rs"))
        .stdout(predicate::str::contains("@src/auth/token_refresh.rs"));

    let events = read_events(project.path());
    assert!(events.iter().any(|e| e.get("files").is_some()));
}
