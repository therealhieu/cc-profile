//! GitHub release metadata and version comparison.

use anyhow::{Context, Result};
use semver::Version;
use serde::Deserialize;

/// Outcome of comparing the running version to a latest release tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionCheckOutcome {
    Current,
    UpdateAvailable { latest: String },
    LookupFailed { message: String },
}

/// Compares `current` to `latest_tag` (e.g. `v0.2.0` or `0.2.0`).
pub fn compare_versions(current: &str, latest_tag: &str) -> Result<VersionCheckOutcome> {
    let current_ver = Version::parse(current).context("invalid current version")?;
    let latest_str = latest_tag.strip_prefix('v').unwrap_or(latest_tag);
    let latest_ver = Version::parse(latest_str).context("invalid latest version")?;

    if latest_ver.pre != semver::Prerelease::EMPTY {
        return Ok(VersionCheckOutcome::Current);
    }

    if latest_ver > current_ver {
        Ok(VersionCheckOutcome::UpdateAvailable {
            latest: latest_ver.to_string(),
        })
    } else {
        Ok(VersionCheckOutcome::Current)
    }
}

#[derive(Debug, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
}

/// Parses GitHub `/releases/latest` JSON body.
pub fn parse_latest_release_json(body: &str) -> Result<GitHubRelease> {
    serde_json::from_str(body).context("invalid GitHub release JSON")
}

/// Selects the release archive asset name for a Rust target triple.
pub fn select_asset_name_for_target(release: &GitHubRelease, target_triple: &str) -> Option<String> {
    let suffix = format!("-{target_triple}.tar.gz");
    release
        .assets
        .iter()
        .find(|a| a.name.ends_with(&suffix))
        .map(|a| a.name.clone())
}

/// Fetches latest release tag via injectable HTTP GET (production uses GitHub API).
pub fn fetch_latest_tag(get_json: impl FnOnce(&str) -> Result<String>) -> Result<String> {
    const URL: &str = "https://api.github.com/repos/therealhieu/cc-profile/releases/latest";
    let body = get_json(URL)?;
    let release = parse_latest_release_json(&body)?;
    Ok(release.tag_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_reports_current_when_equal() {
        let outcome = compare_versions("0.1.0", "v0.1.0").expect("compare");
        assert_eq!(outcome, VersionCheckOutcome::Current);
    }

    #[test]
    fn compare_reports_update_when_outdated() {
        let outcome = compare_versions("0.1.0", "v0.2.0").expect("compare");
        assert_eq!(
            outcome,
            VersionCheckOutcome::UpdateAvailable {
                latest: "0.2.0".to_string()
            }
        );
    }

    #[test]
    fn compare_ignores_prerelease_latest() {
        let outcome = compare_versions("0.1.0", "v0.2.0-beta.1").expect("compare");
        assert_eq!(outcome, VersionCheckOutcome::Current);
    }

    #[test]
    fn compare_fails_closed_on_malformed_latest() {
        let err = compare_versions("0.1.0", "not-a-version").expect_err("malformed");
        assert!(err.to_string().contains("invalid latest version"));
    }

    #[test]
    fn parse_release_json_and_select_asset_by_target() {
        let json = r#"{
            "tag_name": "v0.2.0",
            "assets": [
                {"name": "SHA256SUMS", "browser_download_url": "https://example.com/sums"},
                {"name": "cc-profile-v0.2.0-aarch64-apple-darwin.tar.gz", "browser_download_url": "https://example.com/aarch64"}
            ]
        }"#;
        let release = parse_latest_release_json(json).expect("parse");
        assert_eq!(release.tag_name, "v0.2.0");
        let name = select_asset_name_for_target(&release, "aarch64-apple-darwin")
            .expect("asset");
        assert_eq!(name, "cc-profile-v0.2.0-aarch64-apple-darwin.tar.gz");
    }

    #[test]
    fn fetch_latest_tag_uses_injected_client() {
        let tag = fetch_latest_tag(|_url| {
            Ok(r#"{"tag_name":"v0.3.0","assets":[]}"#.to_string())
        })
        .expect("fetch");
        assert_eq!(tag, "v0.3.0");
    }
}