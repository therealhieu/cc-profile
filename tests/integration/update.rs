use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

fn write_fake_tool(temp: &assert_fs::TempDir, name: &str, log_name: &str) -> PathBuf {
    let log = temp.path().join(log_name);
    let script = format!(
        r#"#!/bin/sh
for arg in "$@"; do
  printf '%s\n' "$arg" >> "{}"
done
exit 0
"#,
        log.display()
    );
    let tool = temp.path().join(name);
    fs::write(&tool, script).expect("script");
    let mut perms = fs::metadata(&tool).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&tool, perms).expect("chmod");
    tool
}

fn read_argv_lines(log: &Path) -> Vec<String> {
    fs::read_to_string(log)
        .expect("log")
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn update_homebrew_delegates_to_fake_brew() {
    let temp = assert_fs::TempDir::new().expect("tempdir");
    let brew = write_fake_tool(&temp, "brew", "brew.log");
    let exe = temp
        .path()
        .join("opt/homebrew/Cellar/cc-profile/0.1.0/bin/cc-profile");
    fs::create_dir_all(exe.parent().expect("parent")).expect("mkdir");
    fs::write(&exe, b"").expect("touch exe");

    Command::cargo_bin("cc-profile")
        .expect("binary")
        .env("CC_PROFILE_UPDATE_LOOKUP", "stub-outdated")
        .env("CC_PROFILE_UPDATE_EXE_PATH", &exe)
        .env("CC_PROFILE_UPDATE_BREW_PATH", &brew)
        .args(["update", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Homebrew"));

    let lines = read_argv_lines(&temp.path().join("brew.log"));
    assert_eq!(
        lines,
        vec!["update", "upgrade", "therealhieu/tap/cc-profile"]
    );
}

#[test]
fn update_cargo_delegates_to_fake_cargo() {
    let temp = assert_fs::TempDir::new().expect("tempdir");
    let cargo = write_fake_tool(&temp, "cargo", "cargo.log");
    let home = temp.path().join("home");
    let exe = home.join(".cargo/bin/cc-profile");
    fs::create_dir_all(exe.parent().expect("parent")).expect("mkdir");
    fs::write(&exe, b"").expect("touch exe");

    let cargo_home = home.join(".cargo");
    Command::cargo_bin("cc-profile")
        .expect("binary")
        .env("HOME", &home)
        .env("CARGO_HOME", &cargo_home)
        .env("CC_PROFILE_UPDATE_LOOKUP", "stub-outdated")
        .env("CC_PROFILE_UPDATE_EXE_PATH", &exe)
        .env("CC_PROFILE_UPDATE_CARGO_PATH", &cargo)
        .args(["update", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cargo"));

    let lines = read_argv_lines(&temp.path().join("cargo.log"));
    assert_eq!(lines, vec!["install", "cc-profile", "--locked", "--force"]);
}
