//! Binary self-update orchestration (check and apply).

use crate::services::release::{self, VersionCheckOutcome};
use anyhow::{Context, Result, bail};
use std::sync::OnceLock;
use std::time::Duration;
use ureq::Agent;

/// Options for an update run.
#[derive(Debug, Clone, Copy)]
pub struct UpdateOptions {
    pub check_only: bool,
    pub skip_confirm: bool,
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
    let current = env!("CARGO_PKG_VERSION");
    if !options.check_only {
        let _ = options.skip_confirm;
        bail!("update without --check is not implemented yet");
    }

    let outcome = release::check_latest_version(current, || lookup_latest_tag(current));
    match outcome {
        VersionCheckOutcome::Current => {
            println!("cc-profile {current} is up to date.");
        }
        VersionCheckOutcome::UpdateAvailable { latest } => {
            println!("cc-profile {latest} is available. Current version: {current}.");
            println!("Run `cc-profile update` to install it.");
        }
        VersionCheckOutcome::LookupFailed { message } => {
            bail!("{message}");
        }
    }
    Ok(())
}

fn default_latest_tag_lookup(current: &str) -> Result<String> {
    #[cfg(debug_assertions)]
    if let Some(tag) = debug_stub_latest_tag(current)? {
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
    let mut response = github_http_agent()
        .get(url)
        .header("User-Agent", "cc-profile")
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(map_ureq_error)
        .with_context(|| format!("request failed for release metadata at {url}"))?;
    response
        .body_mut()
        .read_to_string()
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
}
