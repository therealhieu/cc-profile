use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

fn write_config(temp: &assert_fs::TempDir) {
    temp.child(".cc-profile/config.toml")
        .write_str(
            r#"version = 1
active_profile = "profile-a"

[args]
dangerously_skip_permissions = false

[envs]
HTTP_PROXY = "http://localhost:7890"

[profiles.profile-a]
endpoint = "https://api.anthropic.com"
api_key = "sk-ant-secret"
fable = "claude-fable-5"
opus = "claude-opus-4-8"
sonnet = "claude-sonnet-4-6"
haiku = "claude-haiku-4-5-20251001"

[profiles.profile-b]
endpoint = "https://api.example.com"
api_key = "sk-ant-other"
fable = "custom-fable"
opus = "custom-opus"
sonnet = "custom-sonnet"
haiku = "custom-haiku"
"#,
        )
        .expect("write config");
}

#[test]
fn version_prints_package_version() {
    let version = env!("CARGO_PKG_VERSION");
    Command::cargo_bin("cc-profile")
        .expect("binary exists")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("cc-profile {version}")));
}

#[test]
fn help_lists_core_commands() {
    let mut command = Command::cargo_bin("cc-profile").expect("binary exists");

    command
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("start"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("use"))
        .stdout(predicate::str::contains("show"));
}

#[test]
fn list_marks_active_profile() {
    let temp = assert_fs::TempDir::new().expect("tempdir");
    write_config(&temp);

    Command::cargo_bin("cc-profile")
        .expect("binary exists")
        .env("HOME", temp.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("profile-a  active"))
        .stdout(predicate::str::contains("profile-b"));
}

#[test]
fn use_command_sets_active_profile() {
    let temp = assert_fs::TempDir::new().expect("tempdir");
    write_config(&temp);

    Command::cargo_bin("cc-profile")
        .expect("binary exists")
        .env("HOME", temp.path())
        .args(["use", "profile-b"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Profile \"profile-b\" is now active.",
        ));

    temp.child(".cc-profile/config.toml")
        .assert(predicate::str::contains("active_profile = \"profile-b\""));
}

#[test]
fn show_prints_config_with_unmasked_api_key() {
    let temp = assert_fs::TempDir::new().expect("tempdir");
    write_config(&temp);

    Command::cargo_bin("cc-profile")
        .expect("binary exists")
        .env("HOME", temp.path())
        .arg("show")
        .assert()
        .success()
        .stdout(predicate::str::contains("Config file:"))
        .stdout(predicate::str::contains(".cc-profile/config.toml"))
        .stdout(predicate::str::contains("api_key = \"sk-ant-secret\""));
}

#[test]
fn new_command_creates_profile_and_optionally_sets_active() {
    let temp = assert_fs::TempDir::new().expect("tempdir");

    Command::cargo_bin("cc-profile")
        .expect("binary exists")
        .env("HOME", temp.path())
        .args([
            "new",
            "--name",
            "profile-a",
            "--endpoint",
            "https://api.anthropic.com",
            "--api-key",
            "sk-ant-secret",
            "--fable",
            "claude-fable-5",
            "--opus",
            "claude-opus-4-8",
            "--sonnet",
            "claude-sonnet-4-6",
            "--haiku",
            "claude-haiku-4-5-20251001",
            "--active",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Profile \"profile-a\" saved."))
        .stdout(predicate::str::contains(
            "Profile \"profile-a\" is now active.",
        ));

    temp.child(".cc-profile/config.toml")
        .assert(predicate::str::contains("active_profile = \"profile-a\""))
        .assert(predicate::str::contains("api_key = \"sk-ant-secret\""));
}

#[test]
fn edit_command_updates_profile_fields_and_rename_updates_active() {
    let temp = assert_fs::TempDir::new().expect("tempdir");
    write_config(&temp);

    Command::cargo_bin("cc-profile")
        .expect("binary exists")
        .env("HOME", temp.path())
        .args([
            "edit",
            "profile-a",
            "--rename",
            "profile-c",
            "--endpoint",
            "https://new.example",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Profile \"profile-c\" updated."));

    temp.child(".cc-profile/config.toml")
        .assert(predicate::str::contains("active_profile = \"profile-c\""))
        .assert(predicate::str::contains(
            "endpoint = \"https://new.example\"",
        ));
}

#[test]
fn delete_command_removes_profile_and_clears_active_when_needed() {
    let temp = assert_fs::TempDir::new().expect("tempdir");
    write_config(&temp);

    Command::cargo_bin("cc-profile")
        .expect("binary exists")
        .env("HOME", temp.path())
        .args(["delete", "profile-a"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Profile \"profile-a\" deleted."))
        .stdout(predicate::str::contains(
            "No active profile is currently set.",
        ));

    temp.child(".cc-profile/config.toml")
        .assert(predicate::str::contains("profile-a").not())
        .assert(predicate::str::contains("active_profile").not());
}
