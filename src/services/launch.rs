//! Builds and runs the `claude` process from persisted [`Config`] state.

use crate::config::Config;
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
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
    let Some(active_name) = config.active_profile.as_ref() else {
        bail!("No active profile is set");
    };
    let Some(profile) = config.profiles.get(active_name) else {
        bail!("Active profile '{active_name}' does not exist");
    };

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

/// POSIX single-quote escaping — the only safe way to quote arbitrary values.
///
/// A literal `'` becomes `'\''`: close the quote, emit an escaped quote, reopen the quote.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
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
        .with_context(|| {
            format!(
                "Could not find `{}` on PATH. Please install Claude Code or ensure the `{}` command is available.",
                spec.program, spec.program
            )
        })?;

    if !status.success() {
        bail!("{} exited with status: {}", spec.program, status);
    }

    Ok(())
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

#[cfg(unix)]
fn exec_command_spec(spec: &CommandSpec) -> Result<()> {
    let error = Command::new(&spec.program)
        .args(&spec.args)
        .envs(&spec.envs)
        .exec();

    Err(error).with_context(|| {
        format!(
            "Could not find `{}` on PATH. Please install Claude Code or ensure the `{}` command is available.",
            spec.program, spec.program
        )
    })
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
            "Could not find `cc-profile-definitely-missing-claude-bin` on PATH. Please install Claude Code or ensure the `cc-profile-definitely-missing-claude-bin` command is available."
        ));
    }

    #[test]
    fn start_claude_propagates_build_command_spec_errors() {
        let error = start_claude(&Config::default())
            .expect_err("start should fail before launching claude");
        assert!(error.to_string().contains("No active profile is set"));
    }

    #[test]
    fn shell_quote_wraps_plain_value_in_single_quotes() {
        assert_eq!(shell_quote("sk-ant-secret"), "'sk-ant-secret'");
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
            "ANTHROPIC_API_KEY='sk-ant-profile' \
             ANTHROPIC_BASE_URL='https://api.anthropic.com' \
             ANTHROPIC_DEFAULT_FABLE_MODEL='claude-fable-5' \
             ANTHROPIC_DEFAULT_HAIKU_MODEL='claude-haiku-4-5-20251001' \
             ANTHROPIC_DEFAULT_OPUS_MODEL='claude-opus-4-8' \
             ANTHROPIC_DEFAULT_SONNET_MODEL='claude-sonnet-4-6' \
             HTTP_PROXY='http://localhost:7890' \
             'claude' '--dangerously-skip-permissions'"
        );
    }

    #[test]
    fn render_command_line_emits_no_args_when_spec_args_empty() {
        let spec = build_command_spec(&active_config(false)).expect("spec should build");
        assert!(spec.args.is_empty());

        let rendered = render_command_line(&spec);
        assert!(
            rendered.ends_with("'claude'"),
            "expected rendered line to end with 'claude', got: {rendered}"
        );
    }
}
