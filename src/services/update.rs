//! Binary self-update orchestration (check and apply).

use crate::services::install_method::{
    InstallMethod, InstallPathContext, detect_from_exe_path_with_context,
};
use crate::services::receipt::default_receipt_path_from_env;
use crate::services::update_check_cache::{
    UpdateCheckCache, PASSIVE_CHECK_MIN_INTERVAL, default_cache_path, format_rfc3339_utc,
    is_eligible_for_passive_check, passive_checks_disabled_by_env, read_cache, write_cache,
};
use crate::services::release::{self, GitHubRelease, VersionCheckOutcome};
use crate::services::self_replace::{
    expected_sha256_from_sums, extract_cc_profile_binary_from_tar_gz,
    replace_executable_with_rollback, restore_executable_from_backup, sibling_backup_path,
    smoke_test_binary, verify_archive_sha256,
};
use anyhow::{Context, Result, bail};
use dialoguer::Confirm;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;
use ureq::Agent;

/// Homebrew tap formula upgraded by `update`.
pub const HOMEBREW_FORMULA: &str = "therealhieu/tap/cc-profile";

/// Structured argv for `brew update` (no shell).
pub fn homebrew_update_argv() -> Vec<OsString> {
    vec![OsString::from("update")]
}

/// Structured argv for `brew upgrade <formula>` (no shell).
pub fn homebrew_upgrade_argv() -> Vec<OsString> {
    vec![OsString::from("upgrade"), OsString::from(HOMEBREW_FORMULA)]
}

/// Structured argv for `cargo install cc-profile --locked --force`.
pub fn cargo_reinstall_argv() -> Vec<OsString> {
    vec![
        OsString::from("install"),
        OsString::from("cc-profile"),
        OsString::from("--locked"),
        OsString::from("--force"),
    ]
}

/// Runs `brew` with injectable program name (for tests: `brew` from fake PATH).
pub fn run_brew_sequence(
    brew_program: &Path,
    update_args: &[OsString],
    upgrade_args: &[OsString],
) -> Result<()> {
    let status_update = Command::new(brew_program)
        .args(update_args)
        .status()
        .with_context(|| format!("failed to spawn {}", brew_program.display()))?;
    ensure_success(status_update, "brew update")?;

    let status_upgrade = Command::new(brew_program)
        .args(upgrade_args)
        .status()
        .with_context(|| format!("failed to spawn {}", brew_program.display()))?;
    ensure_success(status_upgrade, "brew upgrade")?;
    Ok(())
}

/// Runs `cargo install` with structured arguments (does not replace the current exe in-place).
pub fn run_cargo_reinstall(cargo_program: &Path, args: &[OsString]) -> Result<()> {
    let status = Command::new(cargo_program)
        .args(args)
        .status()
        .with_context(|| format!("failed to spawn {}", cargo_program.display()))?;
    ensure_success(status, "cargo install cc-profile")?;
    Ok(())
}

fn ensure_success(status: ExitStatus, action: &str) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        let code = status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "signal".to_string());
        bail!("{action} failed with exit status {code}");
    }
}

/// Options for an update run.
#[derive(Debug, Clone, Copy)]
pub struct UpdateOptions {
    pub check_only: bool,
    pub skip_confirm: bool,
}

/// Prompt callback used during interactive update confirmation.
pub type UpdateConfirmFn = Box<dyn FnMut(&str) -> Result<bool>>;

/// Injectable paths and confirmation for update delegation (unit/integration tests).
pub struct UpdateContext {
    pub exe_path: PathBuf,
    pub receipt_path: Option<PathBuf>,
    pub install_paths: InstallPathContext,
    pub brew_program: PathBuf,
    pub cargo_program: PathBuf,
    pub confirm: UpdateConfirmFn,
}

impl UpdateContext {
    /// Resolves the running executable, receipt, and package-manager commands.
    pub fn from_process() -> Result<Self> {
        let exe_path = resolve_current_exe_path()?;
        let receipt_path = default_receipt_path_from_env();
        Ok(Self {
            exe_path,
            receipt_path,
            install_paths: InstallPathContext::from_env(),
            brew_program: PathBuf::from("brew"),
            cargo_program: PathBuf::from("cargo"),
            confirm: Box::new(prompt_confirm),
        })
    }
}

fn resolve_current_exe_path() -> Result<PathBuf> {
    #[cfg(debug_assertions)]
    if let Ok(path) = std::env::var("CC_PROFILE_UPDATE_EXE_PATH") {
        if !path.trim().is_empty() {
            return Ok(PathBuf::from(path));
        }
    }
    std::env::current_exe().context("resolve path to current cc-profile executable")
}

#[cfg(debug_assertions)]
fn apply_debug_program_override(current: PathBuf, env_key: &str) -> PathBuf {
    std::env::var(env_key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or(current)
}

fn prompt_confirm(message: &str) -> Result<bool> {
    Confirm::new()
        .with_prompt(message)
        .default(false)
        .interact()
        .context("read confirmation")
}

/// Prints the passive interactive-mode update notice (does not install).
pub fn print_passive_update_notice(latest: &str) {
    println!("A new cc-profile version is available: {latest}.");
    println!("Run `cc-profile update` to install it.");
}

/// Runs an optional once-per-day passive check; failures and lookup errors do not propagate.
pub fn run_passive_update_check_before_interactive() {
    let _ = run_passive_update_check_injected(
        SystemTime::now(),
        default_cache_path(),
        || {
            release::check_latest_version(env!("CARGO_PKG_VERSION"), || {
                default_latest_tag_lookup(env!("CARGO_PKG_VERSION"))
            })
        },
    );
}

/// Injectable passive check for tests (`cache_path` = `None` skips cache I/O).
pub fn run_passive_update_check_injected(
    now: SystemTime,
    cache_path: Option<PathBuf>,
    lookup: impl FnOnce() -> VersionCheckOutcome,
) -> Result<()> {
    if passive_checks_disabled_by_env() {
        return Ok(());
    }
    let Some(path) = cache_path else {
        return Ok(());
    };
    let existing = read_cache(&path)?;
    if !is_eligible_for_passive_check(
        existing.as_ref(),
        now,
        PASSIVE_CHECK_MIN_INTERVAL,
    ) {
        return Ok(());
    }
    let outcome = lookup();
    let stamp = format_rfc3339_utc(now)?;
    match outcome {
        VersionCheckOutcome::UpdateAvailable { latest } => {
            write_cache(
                &path,
                &UpdateCheckCache {
                    last_checked_at: stamp,
                    latest_seen: Some(latest.clone()),
                },
            )?;
            print_passive_update_notice(&latest);
        }
        VersionCheckOutcome::Current => {
            write_cache(
                &path,
                &UpdateCheckCache {
                    last_checked_at: stamp,
                    latest_seen: None,
                },
            )?;
        }
        VersionCheckOutcome::LookupFailed { .. } => {}
    }
    Ok(())
}

/// Runs update or check-only flow without loading profile config.
pub fn run_update(options: UpdateOptions) -> Result<()> {
    run_update_with_lookup(options, default_latest_tag_lookup)
}

/// Runs update or check-only flow with an injectable latest-tag lookup (for unit tests).
pub fn run_update_with_lookup(
    options: UpdateOptions,
    lookup_latest_tag: impl Fn(&str) -> Result<String>,
) -> Result<()> {
    run_update_injected(
        options,
        lookup_latest_tag,
        || release::fetch_latest_release(fetch_github_release_json),
        fetch_github_release_bytes,
        UpdateContext::from_process,
    )
}

/// Fully injectable update orchestration for tests.
pub fn run_update_injected(
    options: UpdateOptions,
    lookup_latest_tag: impl Fn(&str) -> Result<String>,
    fetch_release: impl FnOnce() -> Result<GitHubRelease>,
    download_bytes: impl Fn(&str) -> Result<Vec<u8>>,
    build_context: impl FnOnce() -> Result<UpdateContext>,
) -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    let outcome = release::check_latest_version(current, || lookup_latest_tag(current));
    match outcome {
        VersionCheckOutcome::Current => {
            println!("cc-profile {current} is up to date.");
            return Ok(());
        }
        VersionCheckOutcome::LookupFailed { message } => {
            bail!("{message}");
        }
        VersionCheckOutcome::UpdateAvailable { latest } => {
            if options.check_only {
                println!("cc-profile {latest} is available. Current version: {current}.");
                println!("Run `cc-profile update` to install it.");
                return Ok(());
            }
            let mut ctx = build_context()?;
            #[cfg(debug_assertions)]
            {
                ctx.brew_program =
                    apply_debug_program_override(ctx.brew_program, "CC_PROFILE_UPDATE_BREW_PATH");
                ctx.cargo_program =
                    apply_debug_program_override(ctx.cargo_program, "CC_PROFILE_UPDATE_CARGO_PATH");
            }
            if !options.skip_confirm {
                let prompt = format!(
                    "Install cc-profile {latest} (current {current})? This will modify your installation."
                );
                if !(ctx.confirm)(&prompt)? {
                    bail!("update cancelled");
                }
            }
            apply_update(&mut ctx, &latest, fetch_release, download_bytes)?;
        }
    }
    Ok(())
}

fn apply_update(
    ctx: &mut UpdateContext,
    latest: &str,
    fetch_release: impl FnOnce() -> Result<GitHubRelease>,
    download_bytes: impl Fn(&str) -> Result<Vec<u8>>,
) -> Result<()> {
    let receipt = ctx.receipt_path.as_deref();
    let method = detect_from_exe_path_with_context(&ctx.exe_path, receipt, &ctx.install_paths);
    match method {
        InstallMethod::Homebrew => {
            run_brew_sequence(
                &ctx.brew_program,
                &homebrew_update_argv(),
                &homebrew_upgrade_argv(),
            )
            .context("Homebrew update failed; try `brew update && brew upgrade therealhieu/tap/cc-profile` manually")?;
            println!("cc-profile upgraded via Homebrew to {latest}.");
        }
        InstallMethod::Cargo => {
            run_cargo_reinstall(&ctx.cargo_program, &cargo_reinstall_argv()).context(
                "Cargo reinstall failed; try `cargo install cc-profile --locked --force` manually",
            )?;
            println!(
                "cc-profile reinstalled via Cargo ({latest} when crates.io matches the release)."
            );
        }
        InstallMethod::Standalone => {
            apply_standalone_update(&ctx.exe_path, latest, fetch_release, download_bytes)
                .context("standalone self-update failed; your existing binary was left in place if replacement did not succeed")?;
        }
        InstallMethod::Unknown => {
            bail!(
                "could not detect install method for {}; use Homebrew, Cargo, or the standalone installer receipt at ~/.cc-profile/install.toml",
                ctx.exe_path.display()
            );
        }
    }
    Ok(())
}

fn apply_standalone_update(
    exe: &Path,
    latest: &str,
    fetch_release: impl FnOnce() -> Result<GitHubRelease>,
    download_bytes: impl Fn(&str) -> Result<Vec<u8>>,
) -> Result<()> {
    let triple = release::host_target_triple()
        .context("standalone update is not supported on this platform")?;
    let release_doc = fetch_release().context("fetch latest GitHub release metadata")?;
    let asset_name = release::select_asset_name_for_target(&release_doc, triple)
        .with_context(|| format!("release has no archive asset for target triple {triple}"))?;
    let archive_url = release::asset_download_url(&release_doc, &asset_name)
        .with_context(|| format!("release metadata has no download URL for {asset_name}"))?;
    let sums_url = release::asset_download_url(&release_doc, "SHA256SUMS")
        .context("release metadata has no SHA256SUMS asset")?;

    ensure_https_download_url(&archive_url)
        .with_context(|| format!("invalid release archive URL {archive_url}"))?;
    ensure_https_download_url(&sums_url)
        .with_context(|| format!("invalid SHA256SUMS URL {sums_url}"))?;

    let archive_bytes = download_bytes(&archive_url)
        .with_context(|| format!("download release archive from {archive_url}"))?;
    let sums_bytes = download_bytes(&sums_url)
        .with_context(|| format!("download SHA256SUMS from {sums_url}"))?;
    let sums_text = String::from_utf8(sums_bytes).context("SHA256SUMS must be valid UTF-8")?;
    let expected =
        expected_sha256_from_sums(&sums_text, &asset_name).context("parse SHA256SUMS")?;
    verify_archive_sha256(&archive_bytes, &expected).context("verify release archive checksum")?;

    let work = TempDir::new().context("create temporary directory for update")?;
    let staged = work.path().join("cc-profile");
    extract_cc_profile_binary_from_tar_gz(&archive_bytes, &staged)
        .context("extract cc-profile binary from release archive")?;

    smoke_test_binary(&staged, |binary| {
        Command::new(binary)
            .arg("--version")
            .output()
            .map_err(Into::into)
    })
    .context("smoke test extracted binary with --version")?;

    let backup = sibling_backup_path(exe);
    replace_executable_with_rollback(exe, &staged, &backup)
        .context("replace installed cc-profile binary")?;

    if let Err(smoke_err) = smoke_test_binary(exe, |binary| {
        Command::new(binary)
            .arg("--version")
            .output()
            .map_err(Into::into)
    }) {
        restore_executable_from_backup(exe, &backup).with_context(|| {
            format!(
                "installed binary failed final --version smoke test; restore from {} failed",
                backup.display()
            )
        })?;
        return Err(smoke_err).context(format!(
            "installed binary at {} failed final --version smoke test after replacement; restored previous binary from {}",
            exe.display(),
            backup.display()
        ));
    }

    println!("cc-profile updated to {latest}.");
    Ok(())
}

fn ensure_https_download_url(url: &str) -> Result<()> {
    if !url.starts_with("https://") {
        bail!("release download URL must use https://");
    }
    Ok(())
}

fn default_latest_tag_lookup(_current: &str) -> Result<String> {
    #[cfg(debug_assertions)]
    if let Some(tag) = debug_stub_latest_tag(_current)? {
        return Ok(tag);
    }
    release::fetch_latest_tag(fetch_github_release_json)
}

/// Debug-build-only env stubs for integration tests against the built binary (no network).
#[cfg(debug_assertions)]
fn debug_stub_latest_tag(current: &str) -> Result<Option<String>> {
    const LOOKUP_ENV: &str = "CC_PROFILE_UPDATE_LOOKUP";
    const LATEST_TAG_ENV: &str = "CC_PROFILE_UPDATE_LATEST_TAG";

    match std::env::var(LOOKUP_ENV).as_deref() {
        Ok("stub-current") => Ok(Some(format!("v{current}"))),
        Ok("stub-outdated") => {
            let tag = match std::env::var(LATEST_TAG_ENV) {
                Ok(value) if value.trim().is_empty() => "v99999.0.0".to_string(),
                Ok(value) => value,
                Err(_) => "v99999.0.0".to_string(),
            };
            Ok(Some(tag))
        }
        Ok("stub-fail") => bail!("simulated release lookup failure"),
        Ok(other) => bail!("unsupported {LOOKUP_ENV} value: {other}"),
        Err(_) => Ok(None),
    }
}

fn github_http_agent() -> &'static Agent {
    static AGENT: OnceLock<Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .http_status_as_error(true)
            .build()
            .into()
    })
}

fn fetch_github_release_json(url: &str) -> Result<String> {
    let bytes = fetch_github_release_bytes(url)?;
    String::from_utf8(bytes).context("release metadata response must be UTF-8")
}

fn fetch_github_release_bytes(url: &str) -> Result<Vec<u8>> {
    let mut response = github_http_agent()
        .get(url)
        .header("User-Agent", "cc-profile")
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(map_ureq_error)
        .with_context(|| format!("request failed for release metadata at {url}"))?;
    response
        .body_mut()
        .read_to_vec()
        .map_err(|e| anyhow::anyhow!(e))
        .context("read release response body")
}

fn map_ureq_error(err: ureq::Error) -> anyhow::Error {
    match err {
        ureq::Error::StatusCode(code) => {
            anyhow::anyhow!("GitHub API returned HTTP status {code}")
        }
        other => anyhow::anyhow!(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use std::path::PathBuf;

    fn read_recorded_argv_lines(log: &Path) -> Vec<String> {
        fs::read_to_string(log)
            .expect("argv log")
            .lines()
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn homebrew_update_argv_is_structured_brew_update() {
        let argv = homebrew_update_argv();
        assert_eq!(argv.len(), 1);
        assert_eq!(argv[0].to_string_lossy(), "update");
    }

    #[test]
    fn homebrew_upgrade_argv_targets_tap_formula() {
        let argv = homebrew_upgrade_argv();
        assert_eq!(argv.len(), 2);
        assert_eq!(argv[0].to_string_lossy(), "upgrade");
        assert_eq!(argv[1].to_string_lossy(), HOMEBREW_FORMULA);
    }

    #[test]
    fn cargo_reinstall_argv_is_locked_force_install() {
        let argv = cargo_reinstall_argv();
        assert_eq!(
            argv.iter()
                .map(|a| a.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
            vec![
                "install".to_string(),
                "cc-profile".to_string(),
                "--locked".to_string(),
                "--force".to_string(),
            ]
        );
    }

    #[test]
    fn run_brew_sequence_uses_fake_brew_on_path() {
        let temp = assert_fs::TempDir::new().expect("tempdir");
        let brew = fake_tool(&temp, "brew", "brew.log");

        run_brew_sequence(&brew, &homebrew_update_argv(), &homebrew_upgrade_argv())
            .expect("brew sequence");

        let lines = read_recorded_argv_lines(&temp.path().join("brew.log"));
        assert_eq!(lines, vec!["update", "upgrade", HOMEBREW_FORMULA]);
    }

    #[test]
    fn run_cargo_reinstall_uses_fake_cargo() {
        let temp = assert_fs::TempDir::new().expect("tempdir");
        let cargo = fake_tool(&temp, "cargo", "cargo.log");

        run_cargo_reinstall(&cargo, &cargo_reinstall_argv()).expect("cargo reinstall");

        let lines = read_recorded_argv_lines(&temp.path().join("cargo.log"));
        assert_eq!(lines, vec!["install", "cc-profile", "--locked", "--force"]);
    }

    #[test]
    fn check_reports_outdated_with_injected_lookup() {
        run_update_with_lookup(
            UpdateOptions {
                check_only: true,
                skip_confirm: false,
            },
            |_current| Ok("v0.2.0".to_string()),
        )
        .expect("check should succeed");
    }

    #[test]
    fn check_fails_closed_when_injected_lookup_fails() {
        let err = run_update_with_lookup(
            UpdateOptions {
                check_only: true,
                skip_confirm: false,
            },
            |_current| bail!("simulated release lookup failure"),
        )
        .expect_err("lookup should fail");
        let message = format!("{err:#}");
        assert!(
            message.contains("simulated"),
            "unexpected error message: {message}"
        );
    }

    fn fake_tool(temp: &assert_fs::TempDir, name: &str, log_name: &str) -> PathBuf {
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
        fs::write(&tool, script).expect("write script");
        let mut perms = fs::metadata(&tool).expect("meta").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&tool, perms).expect("chmod");
        tool
    }

    #[test]
    fn homebrew_update_delegates_with_skip_confirm() {
        let temp = assert_fs::TempDir::new().expect("tempdir");
        let brew = fake_tool(&temp, "brew", "brew.log");
        let exe = PathBuf::from("/opt/homebrew/Cellar/cc-profile/0.1.0/bin/cc-profile");
        run_update_injected(
            UpdateOptions {
                check_only: false,
                skip_confirm: true,
            },
            |_current| Ok("v9.9.9".to_string()),
            || bail!("standalone fetch should not run"),
            |_url| bail!("download should not run"),
            || {
                Ok(UpdateContext {
                    exe_path: exe,
                    receipt_path: None,
                    install_paths: InstallPathContext::default(),
                    brew_program: brew.clone(),
                    cargo_program: PathBuf::from("cargo"),
                    confirm: Box::new(|_| Ok(true)),
                })
            },
        )
        .expect("homebrew update");
        let lines = read_recorded_argv_lines(&temp.path().join("brew.log"));
        assert_eq!(lines, vec!["update", "upgrade", HOMEBREW_FORMULA]);
    }

    #[test]
    fn cargo_update_delegates_with_skip_confirm() {
        let temp = assert_fs::TempDir::new().expect("tempdir");
        let cargo = fake_tool(&temp, "cargo", "cargo.log");
        let exe = PathBuf::from("/Users/dev/.cargo/bin/cc-profile");
        let ctx = InstallPathContext {
            cargo_home: None,
            home_dir: Some(PathBuf::from("/Users/dev")),
        };
        run_update_injected(
            UpdateOptions {
                check_only: false,
                skip_confirm: true,
            },
            |_current| Ok("v9.9.9".to_string()),
            || bail!("release fetch should not run"),
            |_url| bail!("download should not run"),
            || {
                Ok(UpdateContext {
                    exe_path: exe,
                    receipt_path: None,
                    install_paths: ctx,
                    brew_program: PathBuf::from("brew"),
                    cargo_program: cargo.clone(),
                    confirm: Box::new(|_| Ok(true)),
                })
            },
        )
        .expect("cargo update");
        let lines = read_recorded_argv_lines(&temp.path().join("cargo.log"));
        assert_eq!(lines, vec!["install", "cc-profile", "--locked", "--force"]);
    }

    #[test]
    fn update_cancelled_when_confirm_declined() {
        let err = run_update_injected(
            UpdateOptions {
                check_only: false,
                skip_confirm: false,
            },
            |_current| Ok("v9.9.9".to_string()),
            || bail!("no fetch"),
            |_url| bail!("no download"),
            || {
                Ok(UpdateContext {
                    exe_path: PathBuf::from("/opt/homebrew/Cellar/cc-profile/0.1.0/bin/cc-profile"),
                    receipt_path: None,
                    install_paths: InstallPathContext::default(),
                    brew_program: PathBuf::from("brew"),
                    cargo_program: PathBuf::from("cargo"),
                    confirm: Box::new(|_| Ok(false)),
                })
            },
        )
        .expect_err("cancelled");
        assert!(err.to_string().contains("cancelled"), "{err}");
    }

    #[test]
    fn update_failure_lookup_offline() {
        let err = run_update_with_lookup(
            UpdateOptions {
                check_only: true,
                skip_confirm: true,
            },
            |_current| bail!("simulated network timeout"),
        )
        .expect_err("offline");
        assert!(err.to_string().contains("simulated"), "{err}");
    }

    #[test]
    fn update_standalone_rejects_non_https_download_url() {
        let triple = release::host_target_triple().expect("host triple");
        let asset_name = format!("cc-profile-v0.2.0-{triple}.tar.gz");
        let temp = assert_fs::TempDir::new().expect("tempdir");
        let exe = temp.path().join("cc-profile");
        fs::write(&exe, b"original").expect("exe");
        temp.child("install.toml")
            .write_str("method = \"standalone\"\n")
            .expect("receipt");
        let receipt = temp.path().join("install.toml");
        let json = format!(
            r#"{{
            "tag_name": "v0.2.0",
            "assets": [
                {{"name": "SHA256SUMS", "browser_download_url": "http://example.com/SHA256SUMS"}},
                {{"name": "{asset_name}", "browser_download_url": "https://example.com/archive"}}
            ]
        }}"#
        );
        let release = release::parse_latest_release_json(&json).expect("parse");
        let err = run_update_injected(
            UpdateOptions {
                check_only: false,
                skip_confirm: true,
            },
            |_current| Ok("v0.2.0".to_string()),
            || Ok(release),
            |_url| bail!("download should not run"),
            || {
                Ok(UpdateContext {
                    exe_path: exe.clone(),
                    receipt_path: Some(receipt),
                    install_paths: InstallPathContext::default(),
                    brew_program: PathBuf::from("brew"),
                    cargo_program: PathBuf::from("cargo"),
                    confirm: Box::new(|_| Ok(true)),
                })
            },
        )
        .expect_err("http");
        let message = format!("{err:#}").to_ascii_lowercase();
        assert!(message.contains("https"), "unexpected error: {message}");
    }

    #[test]
    fn standalone_checksum_failure_leaves_binary_untouched() {
        let triple = release::host_target_triple().expect("host triple for test");
        let asset_name = format!("cc-profile-v0.2.0-{triple}.tar.gz");
        let temp = assert_fs::TempDir::new().expect("tempdir");
        let exe = temp.path().join("cc-profile");
        fs::write(&exe, b"original").expect("exe");
        temp.child("install.toml")
            .write_str("method = \"standalone\"\n")
            .expect("receipt");
        let receipt = temp.path().join("install.toml");
        let json = format!(
            r#"{{
            "tag_name": "v0.2.0",
            "assets": [
                {{"name": "SHA256SUMS", "browser_download_url": "https://example.com/SHA256SUMS"}},
                {{"name": "{asset_name}", "browser_download_url": "https://example.com/archive"}}
            ]
        }}"#
        );
        let release = release::parse_latest_release_json(&json).expect("parse");
        let err = run_update_injected(
            UpdateOptions {
                check_only: false,
                skip_confirm: true,
            },
            |_current| Ok("v0.2.0".to_string()),
            || Ok(release),
            |url| {
                if url.ends_with("SHA256SUMS") {
                    let wrong = "f".repeat(64);
                    Ok(format!("{wrong}  {asset_name}\n").into_bytes())
                } else {
                    Ok(b"not-the-archive".to_vec())
                }
            },
            || {
                Ok(UpdateContext {
                    exe_path: exe.clone(),
                    receipt_path: Some(receipt.clone()),
                    install_paths: InstallPathContext::default(),
                    brew_program: PathBuf::from("brew"),
                    cargo_program: PathBuf::from("cargo"),
                    confirm: Box::new(|_| Ok(true)),
                })
            },
        )
        .expect_err("checksum");
        let chain = err
            .chain()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let lower = chain.to_ascii_lowercase();
        assert!(
            lower.contains("checksum") || lower.contains("mismatch"),
            "unexpected error chain: {chain}"
        );
        assert_eq!(fs::read(&exe).expect("read"), b"original");
    }

    #[test]
    fn passive_update_check_skips_when_env_disables() {
        use std::sync::{Mutex, MutexGuard};
        static LOCK: Mutex<()> = Mutex::new(());
        let _guard: MutexGuard<'_, ()> = LOCK.lock().expect("lock");
        // SAFETY: serialized by LOCK.
        unsafe {
            std::env::set_var("CC_PROFILE_NO_UPDATE_CHECK", "1");
        }
        let temp = assert_fs::TempDir::new().expect("tempdir");
        let path = temp.path().join("update-check.toml");
        let mut lookup_ran = false;
        run_passive_update_check_injected(
            SystemTime::now(),
            Some(path),
            || {
                lookup_ran = true;
                VersionCheckOutcome::UpdateAvailable {
                    latest: "9.9.9".to_string(),
                }
            },
        )
        .expect("passive");
        assert!(!lookup_ran);
        unsafe {
            std::env::remove_var("CC_PROFILE_NO_UPDATE_CHECK");
        }
    }

    #[test]
    fn passive_update_check_skips_lookup_when_cache_recent() {
        let temp = assert_fs::TempDir::new().expect("tempdir");
        let path = temp.path().join("update-check.toml");
        write_cache(
            &path,
            &UpdateCheckCache {
                last_checked_at: format_rfc3339_utc(SystemTime::now()).expect("stamp"),
                latest_seen: None,
            },
        )
        .expect("write");
        let mut lookup_ran = false;
        run_passive_update_check_injected(
            SystemTime::now(),
            Some(path),
            || {
                lookup_ran = true;
                VersionCheckOutcome::UpdateAvailable {
                    latest: "9.9.9".to_string(),
                }
            },
        )
        .expect("passive");
        assert!(!lookup_ran);
    }

    #[test]
    fn passive_update_check_prints_notice_and_writes_cache() {
        let temp = assert_fs::TempDir::new().expect("tempdir");
        let path = temp.path().join("update-check.toml");
        let now = SystemTime::now();
        run_passive_update_check_injected(
            now,
            Some(path.clone()),
            || {
                VersionCheckOutcome::UpdateAvailable {
                    latest: "0.2.0".to_string(),
                }
            },
        )
        .expect("passive");
        let cache = read_cache(&path).expect("read").expect("cache");
        assert_eq!(cache.latest_seen.as_deref(), Some("0.2.0"));
    }
}
