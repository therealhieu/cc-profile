use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

fn write_fake_brew(temp: &assert_fs::TempDir) -> PathBuf {
    let log = temp.path().join("brew.log");
    let script = format!(
        r#"#!/bin/sh
echo "$@" >> "{}"
exit 0
"#,
        log.display()
    );
    let brew = temp.path().join("brew");
    fs::write(&brew, script).expect("brew script");
    let mut perms = fs::metadata(&brew).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&brew, perms).expect("chmod");
    brew
}

fn write_fake_cargo(temp: &assert_fs::TempDir) -> PathBuf {
    let log = temp.path().join("cargo.log");
    let script = format!(
        r#"#!/bin/sh
echo "$@" >> "{}"
exit 0
"#,
        log.display()
    );
    let cargo = temp.path().join("cargo");
    fs::write(&cargo, script).expect("cargo script");
    let mut perms = fs::metadata(&cargo).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&cargo, perms).expect("chmod");
    cargo
}

#[test]
fn update_homebrew_delegates_to_fake_brew() {
    let temp = assert_fs::TempDir::new().expect("tempdir");
    let brew = write_fake_brew(&temp);
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

    let log = fs::read_to_string(temp.path().join("brew.log")).expect("log");
    assert!(log.contains("update"));
    assert!(log.contains("therealhieu/tap/cc-profile"));
}

#[test]
fn update_cargo_delegates_to_fake_cargo() {
    let temp = assert_fs::TempDir::new().expect("tempdir");
    let cargo = write_fake_cargo(&temp);
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

    let log = fs::read_to_string(temp.path().join("cargo.log")).expect("log");
    assert!(log.contains("install"));
    assert!(log.contains("--locked"));
    assert!(log.contains("--force"));
}