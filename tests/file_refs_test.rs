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

fn write_project_files(project: &TempDir) {
    fs::create_dir_all(project.path().join("src/auth")).unwrap();
    fs::create_dir_all(project.path().join("tests/auth")).unwrap();
    fs::create_dir_all(project.path().join("target/noise")).unwrap();
    fs::write(project.path().join("src/auth/login.rs"), "").unwrap();
    fs::write(project.path().join("src/auth/token_refresh.rs"), "").unwrap();
    fs::write(project.path().join("tests/auth/login_smoke_test.rs"), "").unwrap();
    fs::write(project.path().join("target/noise/generated.rs"), "").unwrap();
}

#[test]
fn files_command_fuzzy_searches_project_files() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    write_project_files(&project);

    cmd(&home)
        .current_dir(project.path())
        .args(["files", "lgn"])
        .assert()
        .success()
        .stdout(predicate::str::contains("@src/auth/login.rs"))
        .stdout(predicate::str::contains("target/noise").not());
}

#[test]
fn add_extracts_fuzzy_at_file_references_from_title() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    write_project_files(&project);

    cmd(&home)
        .current_dir(project.path())
        .args(["add", "Fix redirect in @lgn", "-t", "auth"])
        .assert()
        .success();

    let events = read_log_lines(&project);
    assert_eq!(events[0]["op"], "task.add");
    assert_eq!(events[0]["files"], serde_json::json!(["src/auth/login.rs"]));

    cmd(&home)
        .current_dir(project.path())
        .args(["show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Files: @src/auth/login.rs"));
}

#[test]
fn explicit_file_flag_resolves_fuzzy_query() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    write_project_files(&project);

    cmd(&home)
        .current_dir(project.path())
        .args(["a", "Validate token refresh", "-F", "tokref"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(project.path())
        .args(["show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("@src/auth/token_refresh.rs"));
}

#[test]
fn notes_agent_progress_and_status_reasons_attach_file_mentions() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    write_project_files(&project);

    cmd(&home)
        .current_dir(project.path())
        .args(["add", "Fix auth bug"])
        .assert()
        .success();
    cmd(&home)
        .current_dir(project.path())
        .args(["note", "1", "Repro in @lgnsmk"])
        .assert()
        .success();
    cmd(&home)
        .current_dir(project.path())
        .args(["ag", "1", "claude", "working", "Checking @tokref"])
        .assert()
        .success();
    cmd(&home)
        .current_dir(project.path())
        .args(["fail", "1", "Still failing in @lgnsmk"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(project.path())
        .args(["show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("@tests/auth/login_smoke_test.rs"))
        .stdout(predicate::str::contains("@src/auth/token_refresh.rs"));
}

#[test]
fn ref_and_unref_manage_file_references() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    write_project_files(&project);

    cmd(&home)
        .current_dir(project.path())
        .args(["add", "Fix auth bug"])
        .assert()
        .success();
    cmd(&home)
        .current_dir(project.path())
        .args(["ref", "1", "lgn"])
        .assert()
        .success();
    cmd(&home)
        .current_dir(project.path())
        .args(["show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("@src/auth/login.rs"));
    cmd(&home)
        .current_dir(project.path())
        .args(["unref", "1", "src/auth/login.rs"])
        .assert()
        .success();
    cmd(&home)
        .current_dir(project.path())
        .args(["show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("@src/auth/login.rs").not());
}
