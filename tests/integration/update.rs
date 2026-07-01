use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

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

#[test]
fn update_check_reports_outdated_with_stub_lookup() {
    Command::cargo_bin("cc-profile")
        .expect("binary")
        .env("CC_PROFILE_UPDATE_LOOKUP", "stub-outdated")
        .args(["update", "--check"])
        .assert()
        .success()
        .stdout(predicate::str::contains("is available"))
        .stdout(predicate::str::contains("cc-profile update"));
}

#[test]
fn update_check_reports_current_with_stub_lookup() {
    let version = env!("CARGO_PKG_VERSION");
    Command::cargo_bin("cc-profile")
        .expect("binary")
        .env("CC_PROFILE_UPDATE_LOOKUP", "stub-current")
        .args(["update", "--check"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("{version} is up to date")));
}

#[test]
fn update_does_not_mutate_profile_config() {
    let temp = assert_fs::TempDir::new().expect("tempdir");
    let home = temp.path().join("home");
    let config_dir = home.join(".cc-profile");
    fs::create_dir_all(&config_dir).expect("mkdir");
    let config_path = config_dir.join("config.toml");
    let secret = r#"version = 1
active_profile = "p"

[args]
dangerously_skip_permissions = false

[envs]

[profiles.p]
endpoint = "https://example.com"
api_key = "secret-key"
fable = "f"
opus = "o"
sonnet = "s"
haiku = "h"
"#;
    fs::write(&config_path, secret).expect("config");
    let before = fs::read_to_string(&config_path).expect("read");

    Command::cargo_bin("cc-profile")
        .expect("binary")
        .env("HOME", &home)
        .env("CC_PROFILE_UPDATE_LOOKUP", "stub-outdated")
        .args(["update", "--check"])
        .assert()
        .success();

    let after = fs::read_to_string(&config_path).expect("read");
    assert_eq!(after, before);
}

#[test]
fn update_standalone_replaces_binary_from_fixtures() {
    let triple = match std::env::consts::ARCH {
        "aarch64" if std::env::consts::OS == "macos" => "aarch64-apple-darwin",
        "x86_64" if std::env::consts::OS == "macos" => "x86_64-apple-darwin",
        "x86_64" if std::env::consts::OS == "linux" => "x86_64-unknown-linux-gnu",
        _ => {
            eprintln!("skip update_standalone_replaces_binary_from_fixtures: unsupported host");
            return;
        }
    };

    let temp = assert_fs::TempDir::new().expect("tempdir");
    let home = temp.path().join("home");
    let install_dir = home.join("bin");
    fs::create_dir_all(&install_dir).expect("mkdir");
    let exe = install_dir.join("cc-profile");
    fs::write(&exe, b"#!/bin/sh\necho before\n").expect("old");
    let mut perms = fs::metadata(&exe).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&exe, perms).expect("chmod");

    let receipt_dir = home.join(".cc-profile");
    fs::create_dir_all(&receipt_dir).expect("mkdir");
    fs::write(
        receipt_dir.join("install.toml"),
        "method = \"standalone\"\n",
    )
    .expect("receipt");

    let staging = temp.path().join("staging");
    fs::create_dir_all(&staging).expect("mkdir");
    let new_bin = staging.join("cc-profile");
    fs::write(
        &new_bin,
        b"#!/bin/sh\ncase \"$1\" in --version) echo cc-profile 0.2.0 ;; *) echo ok ;; esac\n",
    )
    .expect("new");
    let mut new_perms = fs::metadata(&new_bin).expect("meta").permissions();
    new_perms.set_mode(0o755);
    fs::set_permissions(&new_bin, new_perms).expect("chmod");

    let archive_name = format!("cc-profile-v0.2.0-{triple}.tar.gz");
    let archive_path = temp.path().join(&archive_name);
    let tar_status = StdCommand::new("tar")
        .arg("czf")
        .arg(&archive_path)
        .arg("-C")
        .arg(&staging)
        .arg("cc-profile")
        .status()
        .expect("tar");
    assert!(tar_status.success(), "tar failed");

    let shasum = StdCommand::new("shasum")
        .args(["-a", "256", archive_path.to_str().expect("utf8")])
        .output()
        .expect("shasum");
    assert!(shasum.status.success());
    let hash_line = String::from_utf8_lossy(&shasum.stdout);
    let hash = hash_line.split_whitespace().next().expect("hash");
    fs::write(
        temp.path().join("SHA256SUMS"),
        format!("{hash}  {archive_name}\n"),
    )
    .expect("sums");

    let json = format!(
        r#"{{
  "tag_name": "v0.2.0",
  "assets": [
    {{ "name": "SHA256SUMS", "browser_download_url": "https://fixtures.example/SHA256SUMS" }},
    {{ "name": "{archive_name}", "browser_download_url": "https://fixtures.example/{archive_name}" }}
  ]
}}"#
    );
    let json_path = temp.path().join("release.json");
    fs::write(&json_path, json).expect("json");

    let fixture_dir = temp.path().to_path_buf();
    Command::cargo_bin("cc-profile")
        .expect("binary")
        .env("HOME", &home)
        .env("CC_PROFILE_UPDATE_LOOKUP", "stub-outdated")
        .env("CC_PROFILE_UPDATE_EXE_PATH", &exe)
        .env("CC_PROFILE_UPDATE_RELEASE_JSON_PATH", &json_path)
        .env("CC_PROFILE_UPDATE_FIXTURE_DIR", &fixture_dir)
        .args(["update", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("updated to"));

    let output = StdCommand::new(&exe)
        .arg("--version")
        .output()
        .expect("run");
    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("0.2.0"),
        "expected updated binary to report 0.2.0"
    );
}
