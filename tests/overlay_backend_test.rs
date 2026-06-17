use assert_cmd::prelude::*;
use predicates::prelude::*;
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

#[test]
fn l_alias_lists_tasks() {
    let home = TempDir::new().unwrap();
    let project = project("auth-service");

    cmd(&home)
        .current_dir(project.path())
        .args(["a", "Fix login redirect"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(project.path())
        .arg("l")
        .assert()
        .success()
        .stdout(predicate::str::contains("Fix login redirect"));
}

#[test]
fn overlay_plain_backend_prints_plain_view() {
    let home = TempDir::new().unwrap();
    let project = project("auth-service");

    cmd(&home)
        .current_dir(project.path())
        .args(["a", "Fix login redirect"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(project.path())
        .args(["overlay", "--backend", "plain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("TODO"))
        .stdout(predicate::str::contains("Fix login redirect"));
}

#[test]
fn overlay_respects_plain_backend_env() {
    let home = TempDir::new().unwrap();
    let project = project("auth-service");

    cmd(&home)
        .current_dir(project.path())
        .args(["a", "Validate token refresh"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(project.path())
        .env("PIN_OVERLAY_BACKEND", "plain")
        .arg("overlay")
        .assert()
        .success()
        .stdout(predicate::str::contains("Validate token refresh"));
}

#[test]
fn overlay_auto_falls_back_to_plain_in_noninteractive_tests() {
    let home = TempDir::new().unwrap();
    let project = project("auth-service");

    cmd(&home)
        .current_dir(project.path())
        .args(["a", "Run auth smoke test"])
        .assert()
        .success();

    cmd(&home)
        .current_dir(project.path())
        .arg("overlay")
        .assert()
        .success()
        .stdout(predicate::str::contains("Run auth smoke test"));
}
