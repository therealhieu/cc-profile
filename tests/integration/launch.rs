use crate::common::{test_claude_shim, test_codex_shim};
use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

fn write_active_profile_config(temp: &assert_fs::TempDir) {
    temp.child(".cc-profile/config.toml")
        .write_str(
            r#"version = 1
active_profile = "profile-a"

[args]
dangerously_skip_permissions = true

[envs]
HTTP_PROXY = "http://localhost:7890"
ANTHROPIC_API_KEY = "custom-env-key"

[profiles.profile-a]
endpoint = "https://api.anthropic.com"
api_key = "sk-ant-profile"
fable = "claude-fable-5"
opus = "claude-opus-4-8"
sonnet = "claude-sonnet-4-6"
haiku = "claude-haiku-4-5-20251001"
"#,
        )
        .expect("write config");
}

#[test]
fn start_launches_claude_with_profile_envs_and_configured_args() {
    let temp = assert_fs::TempDir::new().expect("tempdir");
    let shim_output = temp.child("shim-output.txt");
    write_active_profile_config(&temp);

    let shim = test_claude_shim();
    Command::cargo_bin("cc-profile")
        .expect("binary exists")
        .env("HOME", temp.path())
        .env("CC_PROFILE_CLAUDE_BIN", &shim)
        .env("CC_PROFILE_TEST_CLAUDE_OUTPUT", shim_output.path())
        .arg("start")
        .assert()
        .success();

    shim_output
        .assert(predicate::str::contains("--dangerously-skip-permissions"))
        .assert(predicate::str::contains("HTTP_PROXY=http://localhost:7890"))
        .assert(predicate::str::contains("ANTHROPIC_API_KEY=sk-ant-profile"))
        .assert(predicate::str::contains(
            "ANTHROPIC_DEFAULT_FABLE_MODEL=claude-fable-5",
        ));
}

#[test]
fn start_claude_launches_claude_with_profile_envs_and_configured_args() {
    let temp = assert_fs::TempDir::new().expect("tempdir");
    let shim_output = temp.child("shim-output.txt");
    write_active_profile_config(&temp);

    let shim = test_claude_shim();
    Command::cargo_bin("cc-profile")
        .expect("binary exists")
        .env("HOME", temp.path())
        .env("CC_PROFILE_CLAUDE_BIN", &shim)
        .env("CC_PROFILE_TEST_CLAUDE_OUTPUT", shim_output.path())
        .args(["start", "claude"])
        .assert()
        .success();

    shim_output
        .assert(predicate::str::contains("--dangerously-skip-permissions"))
        .assert(predicate::str::contains("HTTP_PROXY=http://localhost:7890"))
        .assert(predicate::str::contains("ANTHROPIC_API_KEY=sk-ant-profile"))
        .assert(predicate::str::contains(
            "ANTHROPIC_DEFAULT_FABLE_MODEL=claude-fable-5",
        ));
}

#[test]
fn start_codex_syncs_provider_and_launches_with_provider_and_model_args() {
    let home = assert_fs::TempDir::new().expect("home tempdir");
    let codex_home = assert_fs::TempDir::new().expect("codex tempdir");
    let shim_output = home.child("codex-shim-output.txt");
    write_active_profile_config(&home);

    let shim = test_codex_shim();
    Command::cargo_bin("cc-profile")
        .expect("binary exists")
        .env("HOME", home.path())
        .env("CODEX_HOME", codex_home.path())
        .env("CC_PROFILE_CODEX_BIN", &shim)
        .env("CC_PROFILE_TEST_CODEX_OUTPUT", shim_output.path())
        .args(["start", "codex"])
        .assert()
        .success();

    let shim_text = std::fs::read_to_string(shim_output.path()).expect("shim output written");
    assert!(
        shim_text.contains("-c"),
        "shim argv missing -c:\n{shim_text}"
    );
    // Shim captures argv with Debug formatting, so embedded quotes are escaped.
    assert!(
        shim_text.contains(r#"model_provider=\"profile-a\""#),
        "shim argv missing model_provider:\n{shim_text}"
    );
    assert!(
        shim_text.contains("--model"),
        "shim argv missing --model:\n{shim_text}"
    );
    assert!(
        shim_text.contains("claude-opus-4-8"),
        "shim argv missing opus model:\n{shim_text}"
    );
    assert!(
        !shim_text.contains("sk-ant-profile"),
        "shim argv must not contain api key:\n{shim_text}"
    );

    let codex_config = std::fs::read_to_string(codex_home.path().join("config.toml"))
        .expect("codex config written");
    assert!(
        codex_config.contains("[model_providers.profile-a]"),
        "missing provider block:\n{codex_config}"
    );
    assert!(
        codex_config.contains("Bearer sk-ant-profile"),
        "missing bearer auth:\n{codex_config}"
    );
}

#[test]
fn start_codex_errors_without_active_profile() {
    let home = assert_fs::TempDir::new().expect("home tempdir");
    let codex_home = assert_fs::TempDir::new().expect("codex tempdir");
    let shim_output = home.child("codex-shim-output.txt");
    home.child(".cc-profile/config.toml")
        .write_str(
            r#"version = 1

[profiles.profile-a]
endpoint = "https://api.anthropic.com"
api_key = "sk-ant-profile"
fable = "claude-fable-5"
opus = "claude-opus-4-8"
sonnet = "claude-sonnet-4-6"
haiku = "claude-haiku-4-5-20251001"
"#,
        )
        .expect("write config");

    let shim = test_codex_shim();
    Command::cargo_bin("cc-profile")
        .expect("binary exists")
        .env("HOME", home.path())
        .env("CODEX_HOME", codex_home.path())
        .env("CC_PROFILE_CODEX_BIN", &shim)
        .env("CC_PROFILE_TEST_CODEX_OUTPUT", shim_output.path())
        .args(["start", "codex"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No active profile"));

    assert!(
        !shim_output.path().exists(),
        "codex shim should not run when no active profile"
    );
    assert!(
        !codex_home.path().join("config.toml").exists(),
        "codex config should not be written when no active profile"
    );
}

#[test]
fn start_codex_rejects_reserved_active_profile_before_sync() {
    let home = assert_fs::TempDir::new().expect("home tempdir");
    let codex_home = assert_fs::TempDir::new().expect("codex tempdir");
    let shim_output = home.child("codex-shim-output.txt");
    home.child(".cc-profile/config.toml")
        .write_str(
            r#"version = 1
active_profile = "openai"

[profiles.openai]
endpoint = "https://reserved.example"
api_key = "sk-reserved"
fable = "custom-fable"
opus = "custom-opus"
sonnet = "custom-sonnet"
haiku = "custom-haiku"
"#,
        )
        .expect("write config");

    let shim = test_codex_shim();
    Command::cargo_bin("cc-profile")
        .expect("binary exists")
        .env("HOME", home.path())
        .env("CODEX_HOME", codex_home.path())
        .env("CC_PROFILE_CODEX_BIN", &shim)
        .env("CC_PROFILE_TEST_CODEX_OUTPUT", shim_output.path())
        .args(["start", "codex"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("reserved Codex provider id"));

    assert!(
        !shim_output.path().exists(),
        "codex shim should not run for reserved active profile"
    );
    assert!(
        !codex_home.path().join("config.toml").exists(),
        "codex config should not be written for reserved active profile"
    );
}

#[test]
fn start_help_lists_claude_and_codex_targets() {
    Command::cargo_bin("cc-profile")
        .expect("binary exists")
        .args(["start", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("claude"))
        .stdout(predicate::str::contains("codex"));
}
