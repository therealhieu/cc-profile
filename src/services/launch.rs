//! Builds and runs the `claude` process from persisted [`Config`] state.

use crate::config::Config;
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
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

/// Builds a command spec from `config` and runs Claude Code.
///
/// # Errors
///
/// Returns an error from [`build_command_spec`] or [`run_command_spec`].
pub fn start_claude(config: &Config) -> Result<()> {
    let spec = build_command_spec(config)?;
    run_command_spec(&spec)
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
    fn start_claude_propagates_build_command_spec_errors() {
        let error = start_claude(&Config::default())
            .expect_err("start should fail before launching claude");
        assert!(error.to_string().contains("No active profile is set"));
    }
}
