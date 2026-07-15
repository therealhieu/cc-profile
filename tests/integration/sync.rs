use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

/// Writes a two-profile cc-profile config under the tempdir `HOME`.
fn write_config(temp: &assert_fs::TempDir) {
    temp.child(".cc-profile/config.toml")
        .write_str(
            r#"version = 1
active_profile = "profile-a"

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

/// Writes a cc-profile config with a reserved-name (`openai`) profile alongside a
/// normal one. Authored by hand because `cc-profile new` may reject reserved names.
fn write_config_with_reserved(temp: &assert_fs::TempDir) {
    temp.child(".cc-profile/config.toml")
        .write_str(
            r#"version = 1
active_profile = "profile-a"

[profiles.profile-a]
endpoint = "https://api.anthropic.com"
api_key = "sk-ant-secret"
fable = "claude-fable-5"
opus = "claude-opus-4-8"
sonnet = "claude-sonnet-4-6"
haiku = "claude-haiku-4-5-20251001"

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
}

#[test]
fn sync_codex_writes_provider_block_for_profile() {
    let home = assert_fs::TempDir::new().expect("home tempdir");
    write_config(&home);
    let codex_home = assert_fs::TempDir::new().expect("codex tempdir");

    Command::cargo_bin("cc-profile")
        .expect("binary exists")
        .env("HOME", home.path())
        .env("CODEX_HOME", codex_home.path())
        .args(["sync", "codex"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Synced"))
        .stdout(predicate::str::contains(
            codex_home.path().join("config.toml").display().to_string(),
        ));

    let written = std::fs::read_to_string(codex_home.path().join("config.toml"))
        .expect("codex config written");
    assert!(
        written.contains("[model_providers.profile-a]"),
        "missing provider block:\n{written}"
    );
    assert!(
        written.contains(r#"base_url = "https://api.anthropic.com""#),
        "missing base_url:\n{written}"
    );
    assert!(
        written.contains(r#"Authorization = "Bearer sk-ant-secret""#),
        "missing Authorization header:\n{written}"
    );
}

#[test]
fn sync_codex_skips_reserved_profile_and_still_succeeds() {
    let home = assert_fs::TempDir::new().expect("home tempdir");
    write_config_with_reserved(&home);
    let codex_home = assert_fs::TempDir::new().expect("codex tempdir");

    Command::cargo_bin("cc-profile")
        .expect("binary exists")
        .env("HOME", home.path())
        .env("CODEX_HOME", codex_home.path())
        .args(["sync", "codex"])
        .assert()
        .success()
        .stderr(predicate::str::contains(r#"Skipped profile "openai""#));

    let written = std::fs::read_to_string(codex_home.path().join("config.toml"))
        .expect("codex config written");
    assert!(
        written.contains("[model_providers.profile-a]"),
        "normal profile should sync:\n{written}"
    );
    assert!(
        !written.contains("[model_providers.openai]"),
        "reserved provider block should not be written:\n{written}"
    );
    assert!(
        !written.contains("reserved.example"),
        "reserved profile endpoint should not be written:\n{written}"
    );
}

#[test]
fn sync_codex_preserves_existing_foreign_content() {
    let home = assert_fs::TempDir::new().expect("home tempdir");
    write_config(&home);
    let codex_home = assert_fs::TempDir::new().expect("codex tempdir");
    let existing = "# comment\n\
model = \"gpt-5\"\n\
\n\
[model_providers.other]\n\
name = \"Other\"\n\
base_url = \"https://other.example\"\n";
    codex_home
        .child("config.toml")
        .write_str(existing)
        .expect("write existing codex config");

    Command::cargo_bin("cc-profile")
        .expect("binary exists")
        .env("HOME", home.path())
        .env("CODEX_HOME", codex_home.path())
        .args(["sync", "codex"])
        .assert()
        .success();

    let written = std::fs::read_to_string(codex_home.path().join("config.toml"))
        .expect("codex config written");
    assert!(written.contains("# comment"), "comment lost:\n{written}");
    assert!(
        written.contains(r#"model = "gpt-5""#),
        "top-level key lost:\n{written}"
    );
    assert!(
        written.contains("[model_providers.other]"),
        "foreign provider lost:\n{written}"
    );
    assert!(
        written.contains("[model_providers.profile-a]"),
        "new provider not added:\n{written}"
    );
}
