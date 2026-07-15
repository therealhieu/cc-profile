//! Builds and runs the `claude` process from persisted [`Config`] state.

use crate::config::{Config, Profile};
use crate::services::sync_codex;
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

/// Resolved program, arguments, and environment for launching Claude Code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub envs: BTreeMap<String, String>,
}

/// Builds a [`CommandSpec`] from the active profile, global env vars, and Claude args.
///
/// Profile-specific `ANTHROPIC_*` values are applied after global envs so the active profile wins.
///
/// # Errors
///
/// Returns an error when no active profile is set or the active profile name is missing from
/// [`Config::profiles`].
pub fn build_command_spec(config: &Config) -> Result<CommandSpec> {
    let program_override = std::env::var("CC_PROFILE_CLAUDE_BIN").ok();
    build_command_spec_with_program(config, program_override)
}

/// Like [`build_command_spec`], but uses `program_override` when set instead of reading
/// `CC_PROFILE_CLAUDE_BIN` from the environment.
pub(crate) fn build_command_spec_with_program(
    config: &Config,
    program_override: Option<String>,
) -> Result<CommandSpec> {
    let (_active_name, profile) = resolve_active_profile(config)?;

    let mut envs = config.envs.clone();
    envs.insert("ANTHROPIC_BASE_URL".to_string(), profile.endpoint.clone());
    envs.insert("ANTHROPIC_API_KEY".to_string(), profile.api_key.clone());

    let model_envs = [
        ("ANTHROPIC_DEFAULT_FABLE_MODEL", &profile.fable),
        ("ANTHROPIC_DEFAULT_OPUS_MODEL", &profile.opus),
        ("ANTHROPIC_DEFAULT_SONNET_MODEL", &profile.sonnet),
        ("ANTHROPIC_DEFAULT_HAIKU_MODEL", &profile.haiku),
    ];
    for (key, value) in model_envs {
        envs.insert(key.to_string(), value.clone());
    }

    let mut args = Vec::new();
    if config.args.dangerously_skip_permissions {
        args.push("--dangerously-skip-permissions".to_string());
    }

    let program = program_override.unwrap_or_else(|| "claude".to_string());

    Ok(CommandSpec {
        program,
        args,
        envs,
    })
}

/// Builds a [`CommandSpec`] for Codex from the active profile's name and opus model.
///
/// Program comes from `CC_PROFILE_CODEX_BIN` when set, otherwise `"codex"`. Args are exactly
/// `["-c", "model_provider=\"<name>\"", "--model", "<opus>"]` with an empty env map.
///
/// # Errors
///
/// Returns an error when no active profile is set or the active profile name is missing from
/// [`Config::profiles`].
pub fn build_codex_command_spec(config: &Config) -> Result<CommandSpec> {
    let program_override = std::env::var("CC_PROFILE_CODEX_BIN").ok();
    build_codex_command_spec_with_program(config, program_override)
}

/// Like [`build_codex_command_spec`], but uses `program_override` when set instead of reading
/// `CC_PROFILE_CODEX_BIN` from the environment.
pub(crate) fn build_codex_command_spec_with_program(
    config: &Config,
    program_override: Option<String>,
) -> Result<CommandSpec> {
    let (name, profile) = resolve_active_profile(config)?;
    Ok(CommandSpec {
        program: program_override.unwrap_or_else(|| "codex".into()),
        args: vec![
            "-c".into(),
            format!("model_provider=\"{name}\""),
            "--model".into(),
            profile.opus.clone(),
        ],
        envs: BTreeMap::new(),
    })
}

fn resolve_active_profile(config: &Config) -> Result<(&str, &Profile)> {
    let Some(active_name) = config.active_profile.as_ref() else {
        bail!("No active profile is set");
    };
    let Some(profile) = config.profiles.get(active_name) else {
        bail!("Active profile '{active_name}' does not exist");
    };
    Ok((active_name.as_str(), profile))
}

/// POSIX shell quoting: returns `value` unquoted when it consists only of characters
/// no shell interprets, otherwise wraps it in single quotes.
///
/// The safe set matches Python's `shlex.quote` (`[A-Za-z0-9_@%+=:,./-]`), so barewords like
/// `claude`, `--dangerously-skip-permissions`, and `https://api.anthropic.com` stay unquoted while
/// values with globs, spaces, or quotes are escaped. A literal `'` becomes `'\''`: close the quote,
/// emit an escaped quote, reopen the quote.
fn shell_quote(value: &str) -> String {
    let is_safe = !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_@%+=:,./-".contains(&b));
    if is_safe {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', r"'\''"))
    }
}

/// Renders `spec` as a single copy-pasteable shell command line:
/// `KEY='v' KEY2='v2' <program> <arg>...`, envs in `spec.envs` (sorted) order.
pub fn render_command_line(spec: &CommandSpec) -> String {
    let mut parts: Vec<String> = spec
        .envs
        .iter()
        .map(|(k, v)| format!("{k}={}", shell_quote(v)))
        .collect();
    parts.push(shell_quote(&spec.program));
    parts.extend(spec.args.iter().map(|a| shell_quote(a)));
    parts.join(" ")
}

/// Spawns `spec.program` with `spec.args` and `spec.envs`, waiting for exit.
///
/// # Errors
///
/// Returns an error when the program is not found on `PATH` or exits with a non-success status.
pub fn run_command_spec(spec: &CommandSpec) -> Result<()> {
    let status = Command::new(&spec.program)
        .args(&spec.args)
        .envs(&spec.envs)
        .status()
        .with_context(|| missing_program_message(&spec.program))?;

    if !status.success() {
        bail!("{} exited with status: {}", spec.program, status);
    }

    Ok(())
}

fn missing_program_message(program: &str) -> String {
    format!(
        "Could not find `{program}` on PATH. Please install it or ensure the `{program}` command is available."
    )
}

/// Builds a command spec from `config` and replaces this process with Claude Code.
///
/// # Errors
///
/// Returns an error from [`build_command_spec`] or when executing Claude Code fails before the
/// process image is replaced.
pub fn start_claude(config: &Config) -> Result<()> {
    start_claude_with_launcher(config, exec_command_spec)
}

fn start_claude_with_launcher<F>(config: &Config, launch: F) -> Result<()>
where
    F: FnOnce(&CommandSpec) -> Result<()>,
{
    let spec = build_command_spec(config)?;
    launch(&spec)
}

/// Syncs the active profile into Codex config, then replaces this process with Codex.
///
/// # Errors
///
/// Returns an error when no active profile is set, the active profile is a reserved Codex
/// provider id, sync fails, building the command fails, or launching Codex fails before the
/// process image is replaced.
pub fn start_codex(config: &Config) -> Result<()> {
    let path = sync_codex::codex_config_path()?;
    start_codex_with_path_and_launcher(config, &path, exec_command_spec)
}

/// Path- and launcher-injectable seam for [`start_codex`].
///
/// Resolves the active profile, rejects reserved provider ids before any sync or launch, syncs
/// profiles into `codex_path`, builds the Codex command, then invokes `launch`.
pub(crate) fn start_codex_with_path_and_launcher<F>(
    config: &Config,
    codex_path: &Path,
    launch: F,
) -> Result<()>
where
    F: FnOnce(&CommandSpec) -> Result<()>,
{
    let (name, _) = resolve_active_profile(config)?;
    if sync_codex::is_reserved_provider_id(name) {
        bail!("Cannot start Codex: profile '{name}' is a reserved Codex provider id");
    }
    let _skipped = sync_codex::sync(config, codex_path)?;
    launch(&build_codex_command_spec(config)?)
}

#[cfg(unix)]
fn exec_command_spec(spec: &CommandSpec) -> Result<()> {
    let error = Command::new(&spec.program)
        .args(&spec.args)
        .envs(&spec.envs)
        .exec();

    Err(error).with_context(|| missing_program_message(&spec.program))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Args, Profile};

    fn active_config(skip_permissions: bool) -> Config {
        Config {
            active_profile: Some("profile-a".to_string()),
            args: Args {
                dangerously_skip_permissions: skip_permissions,
            },
            envs: BTreeMap::from([
                (
                    "ANTHROPIC_API_KEY".to_string(),
                    "custom-env-key".to_string(),
                ),
                (
                    "HTTP_PROXY".to_string(),
                    "http://localhost:7890".to_string(),
                ),
            ]),
            profiles: BTreeMap::from([(
                "profile-a".to_string(),
                Profile::builder()
                    .endpoint("https://api.anthropic.com".to_string())
                    .api_key("sk-ant-profile".to_string())
                    .fable("claude-fable-5".to_string())
                    .opus("claude-opus-4-8".to_string())
                    .sonnet("claude-sonnet-4-6".to_string())
                    .haiku("claude-haiku-4-5-20251001".to_string())
                    .build(),
            )]),
            ..Config::default()
        }
    }

    #[test]
    fn build_command_spec_uses_program_override_when_set() {
        let spec = build_command_spec_with_program(
            &active_config(false),
            Some("/tmp/custom-claude".to_string()),
        )
        .expect("spec should build");
        assert_eq!(spec.program, "/tmp/custom-claude");
    }

    #[test]
    fn build_codex_command_spec_uses_active_profile_provider_and_opus() {
        let spec = build_codex_command_spec(&active_config(false)).expect("spec should build");

        assert_eq!(spec.program, "codex");
        assert_eq!(
            spec.args,
            vec![
                "-c".to_string(),
                "model_provider=\"profile-a\"".to_string(),
                "--model".to_string(),
                "claude-opus-4-8".to_string(),
            ]
        );
        assert!(spec.envs.is_empty());
    }

    #[test]
    fn build_codex_command_spec_uses_program_override_when_set() {
        let spec = build_codex_command_spec_with_program(
            &active_config(false),
            Some("/tmp/custom-codex".to_string()),
        )
        .expect("spec should build");
        assert_eq!(spec.program, "/tmp/custom-codex");
    }

    #[test]
    fn build_codex_command_spec_errors_when_active_profile_is_missing() {
        let error = build_codex_command_spec(&Config::default())
            .expect_err("missing active profile should fail");
        assert!(error.to_string().contains("No active profile is set"));
    }

    #[test]
    fn build_codex_command_spec_errors_when_active_profile_references_missing_profile() {
        let config = Config {
            active_profile: Some("missing-profile".to_string()),
            ..Config::default()
        };
        let error =
            build_codex_command_spec(&config).expect_err("missing profile entry should fail");
        assert!(error.to_string().contains("does not exist"));
    }

    #[test]
    fn build_codex_command_spec_quotes_provider_name_for_toml_typing() {
        let config = Config {
            active_profile: Some("true".to_string()),
            profiles: BTreeMap::from([(
                "true".to_string(),
                Profile::builder()
                    .endpoint("https://api.anthropic.com".to_string())
                    .api_key("sk-ant-profile".to_string())
                    .fable("claude-fable-5".to_string())
                    .opus("claude-opus-4-8".to_string())
                    .sonnet("claude-sonnet-4-6".to_string())
                    .haiku("claude-haiku-4-5-20251001".to_string())
                    .build(),
            )]),
            ..Config::default()
        };

        let spec = build_codex_command_spec(&config).expect("spec should build");
        assert_eq!(
            spec.args,
            vec![
                "-c".to_string(),
                "model_provider=\"true\"".to_string(),
                "--model".to_string(),
                "claude-opus-4-8".to_string(),
            ]
        );
    }

    #[test]
    fn build_command_spec_uses_active_profile_envs_after_global_envs() {
        let spec = build_command_spec(&active_config(false)).expect("spec should build");

        assert_eq!(spec.program, "claude");
        assert_eq!(
            spec.envs.get("HTTP_PROXY").map(String::as_str),
            Some("http://localhost:7890")
        );
        assert_eq!(
            spec.envs.get("ANTHROPIC_API_KEY").map(String::as_str),
            Some("sk-ant-profile")
        );
        assert_eq!(
            spec.envs.get("ANTHROPIC_BASE_URL").map(String::as_str),
            Some("https://api.anthropic.com")
        );
        assert_eq!(
            spec.envs
                .get("ANTHROPIC_DEFAULT_FABLE_MODEL")
                .map(String::as_str),
            Some("claude-fable-5")
        );
        assert_eq!(
            spec.envs
                .get("ANTHROPIC_DEFAULT_OPUS_MODEL")
                .map(String::as_str),
            Some("claude-opus-4-8")
        );
        assert_eq!(
            spec.envs
                .get("ANTHROPIC_DEFAULT_SONNET_MODEL")
                .map(String::as_str),
            Some("claude-sonnet-4-6")
        );
        assert_eq!(
            spec.envs
                .get("ANTHROPIC_DEFAULT_HAIKU_MODEL")
                .map(String::as_str),
            Some("claude-haiku-4-5-20251001")
        );
    }

    #[test]
    fn build_command_spec_adds_skip_permissions_flag_only_when_enabled() {
        assert!(
            build_command_spec(&active_config(false))
                .expect("spec")
                .args
                .is_empty()
        );
        assert_eq!(
            build_command_spec(&active_config(true)).expect("spec").args,
            vec!["--dangerously-skip-permissions"]
        );
    }

    #[test]
    fn build_command_spec_errors_when_active_profile_is_missing() {
        let error =
            build_command_spec(&Config::default()).expect_err("missing active profile should fail");
        assert!(error.to_string().contains("No active profile is set"));
    }

    #[test]
    fn build_command_spec_errors_when_active_profile_references_missing_profile() {
        let config = Config {
            active_profile: Some("missing-profile".to_string()),
            ..Config::default()
        };
        let error = build_command_spec(&config).expect_err("missing profile entry should fail");
        assert!(
            error
                .to_string()
                .contains("Active profile 'missing-profile' does not exist")
        );
    }

    #[test]
    fn start_claude_with_launcher_invokes_launcher_with_built_spec() {
        let expected_spec = build_command_spec(&active_config(true)).expect("spec should build");
        let captured = std::cell::RefCell::new(None);

        start_claude_with_launcher(&active_config(true), |spec| {
            *captured.borrow_mut() = Some(spec.clone());
            Ok(())
        })
        .expect("launcher should receive spec");

        assert_eq!(captured.into_inner(), Some(expected_spec));
    }

    #[test]
    fn exec_command_spec_returns_context_when_exec_fails() {
        let spec = CommandSpec {
            program: "cc-profile-definitely-missing-claude-bin".to_string(),
            args: Vec::new(),
            envs: BTreeMap::new(),
        };

        let error = exec_command_spec(&spec).expect_err("exec should fail for missing program");

        assert!(error.to_string().contains(
            "Could not find `cc-profile-definitely-missing-claude-bin` on PATH. Please install it or ensure the `cc-profile-definitely-missing-claude-bin` command is available."
        ));
        assert!(!error.to_string().contains("Claude Code"));
    }

    #[test]
    fn exec_command_spec_missing_program_message_is_program_agnostic_for_codex() {
        let spec = CommandSpec {
            program: "cc-profile-definitely-missing-codex-bin".to_string(),
            args: Vec::new(),
            envs: BTreeMap::new(),
        };

        let error = exec_command_spec(&spec).expect_err("exec should fail for missing program");
        let message = error.to_string();

        assert!(message.contains(
            "Could not find `cc-profile-definitely-missing-codex-bin` on PATH. Please install it or ensure the `cc-profile-definitely-missing-codex-bin` command is available."
        ));
        assert!(!message.contains("Claude Code"));
    }

    #[test]
    fn start_claude_propagates_build_command_spec_errors() {
        let error = start_claude(&Config::default())
            .expect_err("start should fail before launching claude");
        assert!(error.to_string().contains("No active profile is set"));
    }

    #[test]
    fn start_codex_happy_path_syncs_provider_and_launches_spec() {
        let dir = tempfile::tempdir().expect("tempdir");
        let codex_path = dir.path().join("config.toml");
        let expected_spec =
            build_codex_command_spec(&active_config(false)).expect("spec should build");
        let captured = std::cell::RefCell::new(None);

        start_codex_with_path_and_launcher(&active_config(false), &codex_path, |spec| {
            *captured.borrow_mut() = Some(spec.clone());
            Ok(())
        })
        .expect("start_codex should succeed");

        let launched = captured
            .into_inner()
            .expect("launcher should receive a spec");
        assert_eq!(launched, expected_spec);
        assert!(
            launched
                .args
                .iter()
                .all(|arg| !arg.contains("sk-ant-profile")),
            "api key must not appear on argv: {:?}",
            launched.args
        );

        let written = std::fs::read_to_string(&codex_path).expect("codex config written");
        assert!(
            written.contains("[model_providers.profile-a]"),
            "expected provider block, got:\n{written}"
        );
        assert!(
            written.contains("Bearer sk-ant-profile"),
            "expected Bearer token, got:\n{written}"
        );
    }

    #[test]
    fn start_codex_missing_active_profile_fails_before_launch_and_write() {
        let dir = tempfile::tempdir().expect("tempdir");
        let codex_path = dir.path().join("config.toml");
        let launched = std::cell::Cell::new(false);

        let error = start_codex_with_path_and_launcher(&Config::default(), &codex_path, |_spec| {
            launched.set(true);
            Ok(())
        })
        .expect_err("missing active profile should fail");

        assert!(error.to_string().contains("No active profile is set"));
        assert!(!launched.get(), "launcher must not be called");
        assert!(
            !codex_path.exists(),
            "codex path must remain absent when active profile is missing"
        );
    }

    #[test]
    fn start_codex_reserved_active_profile_fails_before_sync_and_launch() {
        let dir = tempfile::tempdir().expect("tempdir");
        let codex_path = dir.path().join("config.toml");
        let launched = std::cell::Cell::new(false);
        let config = Config {
            active_profile: Some("openai".to_string()),
            profiles: BTreeMap::from([(
                "openai".to_string(),
                Profile::builder()
                    .endpoint("https://api.openai.com".to_string())
                    .api_key("sk-openai".to_string())
                    .fable("fable".to_string())
                    .opus("opus".to_string())
                    .sonnet("sonnet".to_string())
                    .haiku("haiku".to_string())
                    .build(),
            )]),
            ..Config::default()
        };

        let error = start_codex_with_path_and_launcher(&config, &codex_path, |_spec| {
            launched.set(true);
            Ok(())
        })
        .expect_err("reserved active profile should fail");

        assert!(
            error
                .to_string()
                .contains("Cannot start Codex: profile 'openai' is a reserved Codex provider id")
        );
        assert!(!launched.get(), "launcher must not be called");
        assert!(
            !codex_path.exists(),
            "reserved active profile must not write codex config"
        );
    }

    #[test]
    fn start_codex_sync_error_propagates_and_skips_launch() {
        let dir = tempfile::tempdir().expect("tempdir");
        let codex_path = dir.path().join("config.toml");
        std::fs::write(&codex_path, "model_providers = \"not-a-table\"\n")
            .expect("seed invalid toml");
        let before = std::fs::read_to_string(&codex_path).expect("read seeded file");
        let launched = std::cell::Cell::new(false);

        let error =
            start_codex_with_path_and_launcher(&active_config(false), &codex_path, |_spec| {
                launched.set(true);
                Ok(())
            })
            .expect_err("invalid existing toml should fail sync");

        assert!(
            error.to_string().contains("model_providers")
                || error
                    .chain()
                    .any(|cause| cause.to_string().contains("model_providers")),
            "expected model_providers sync error, got: {error:#}"
        );
        assert!(
            !launched.get(),
            "launcher must not be called after sync error"
        );
        let after = std::fs::read_to_string(&codex_path).expect("read after failed sync");
        assert_eq!(before, after, "failed sync must not rewrite invalid config");
    }

    #[test]
    fn shell_quote_leaves_safe_value_unquoted() {
        assert_eq!(shell_quote("sk-ant-secret"), "sk-ant-secret");
        assert_eq!(
            shell_quote("https://api.anthropic.com"),
            "https://api.anthropic.com"
        );
    }

    #[test]
    fn shell_quote_quotes_value_with_shell_special_chars() {
        assert_eq!(
            shell_quote("claude-opus-4.8-thinking[1m]"),
            "'claude-opus-4.8-thinking[1m]'"
        );
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn shell_quote_escapes_embedded_single_quote() {
        assert_eq!(shell_quote("a'b"), r"'a'\''b'");
    }

    #[test]
    fn render_command_line_renders_sorted_envs_program_and_args() {
        let spec = build_command_spec(&active_config(true)).expect("spec should build");

        assert_eq!(
            render_command_line(&spec),
            "ANTHROPIC_API_KEY=sk-ant-profile \
             ANTHROPIC_BASE_URL=https://api.anthropic.com \
             ANTHROPIC_DEFAULT_FABLE_MODEL=claude-fable-5 \
             ANTHROPIC_DEFAULT_HAIKU_MODEL=claude-haiku-4-5-20251001 \
             ANTHROPIC_DEFAULT_OPUS_MODEL=claude-opus-4-8 \
             ANTHROPIC_DEFAULT_SONNET_MODEL=claude-sonnet-4-6 \
             HTTP_PROXY=http://localhost:7890 \
             claude --dangerously-skip-permissions"
        );
    }

    #[test]
    fn render_command_line_emits_no_args_when_spec_args_empty() {
        let spec = build_command_spec(&active_config(false)).expect("spec should build");
        assert!(spec.args.is_empty());

        let rendered = render_command_line(&spec);
        assert!(
            rendered.ends_with("claude"),
            "expected rendered line to end with claude, got: {rendered}"
        );
    }
}
