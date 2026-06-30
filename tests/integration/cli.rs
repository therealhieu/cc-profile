use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

fn write_config(temp: &assert_fs::TempDir) {
    temp.child(".cc-profile")
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

    temp.child(".cc-profile")
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
        .stdout(predicate::str::contains("api_key = \"sk-ant-secret\""));
}
