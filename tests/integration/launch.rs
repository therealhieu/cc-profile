use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn start_launches_claude_with_profile_envs_and_configured_args() {
    let temp = assert_fs::TempDir::new().expect("tempdir");
    let shim_output = temp.child("shim-output.txt");
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

    let shim = assert_cmd::cargo::cargo_bin("cc-profile-test-claude");
    Command::cargo_bin("cc-profile")
        .expect("binary exists")
        .env("HOME", temp.path())
        .env("CC_PROFILE_CLAUDE_BIN", shim)
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
