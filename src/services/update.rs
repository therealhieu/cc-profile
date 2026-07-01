//! Binary self-update orchestration (check and apply).

use crate::services::release::{self, VersionCheckOutcome};
use anyhow::{Context, Result, bail};

/// Options for an update run.
#[derive(Debug, Clone, Copy)]
pub struct UpdateOptions {
    pub check_only: bool,
    pub skip_confirm: bool,
}

/// Runs update or check-only flow without loading profile config.
pub fn run_update(options: UpdateOptions) -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    if !options.check_only {
        let _ = options.skip_confirm;
        bail!("update without --check is not implemented yet");
    }

    let latest_tag = lookup_latest_tag(current).context("update lookup failed")?;
    match release::compare_versions(current, &latest_tag)? {
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

fn lookup_latest_tag(current: &str) -> Result<String> {
    match std::env::var("CC_PROFILE_UPDATE_LOOKUP").as_deref() {
        Ok("stub-current") => Ok(format!("v{current}")),
        Ok("stub-outdated") => Ok(
            std::env::var("CC_PROFILE_UPDATE_LATEST_TAG")
                .unwrap_or_else(|_| "v99.0.0".to_string()),
        ),
        Ok("stub-fail") => bail!("simulated release lookup failure"),
        Ok(other) => bail!("unsupported CC_PROFILE_UPDATE_LOOKUP value: {other}"),
        Err(_) => release::fetch_latest_tag(fetch_github_release_json),
    }
}

fn fetch_github_release_json(url: &str) -> Result<String> {
    let mut response = ureq::get(url)
        .header("User-Agent", "cc-profile")
        .call()
        .with_context(|| "request failed for release metadata".to_string())?;
    response
        .body_mut()
        .read_to_string()
        .map_err(|e| anyhow::anyhow!(e))
        .context("read release response body")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn lock_update_env_tests() -> MutexGuard<'static, ()> {
        ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn check_reports_outdated_with_stub_outdated() {
        let _guard = lock_update_env_tests();
        unsafe {
            std::env::set_var("CC_PROFILE_UPDATE_LOOKUP", "stub-outdated");
            std::env::set_var("CC_PROFILE_UPDATE_LATEST_TAG", "v0.2.0");
        }
        let result = run_update(UpdateOptions {
            check_only: true,
            skip_confirm: false,
        });
        unsafe {
            std::env::remove_var("CC_PROFILE_UPDATE_LOOKUP");
            std::env::remove_var("CC_PROFILE_UPDATE_LATEST_TAG");
        }
        result.expect("check should succeed");
    }

    #[test]
    fn check_fails_closed_when_stub_fail() {
        let _guard = lock_update_env_tests();
        unsafe {
            std::env::set_var("CC_PROFILE_UPDATE_LOOKUP", "stub-fail");
        }
        let err = run_update(UpdateOptions {
            check_only: true,
            skip_confirm: false,
        })
        .expect_err("lookup should fail");
        unsafe {
            std::env::remove_var("CC_PROFILE_UPDATE_LOOKUP");
        }
        let message = format!("{err:#}");
        assert!(
            message.contains("simulated"),
            "unexpected error message: {message}"
        );
    }
}