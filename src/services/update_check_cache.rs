//! Passive update-check cache (`update-check.toml`), separate from profile config.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Minimum interval between passive GitHub lookups (once per day).
pub const PASSIVE_CHECK_MIN_INTERVAL: Duration = Duration::from_secs(86_400);

/// On-disk cache for passive update checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCheckCache {
    pub last_checked_unix: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_seen: Option<String>,
}

/// Whole seconds since the Unix epoch.
///
/// # Errors
/// Returns an error when `time` is before `UNIX_EPOCH`.
pub(crate) fn unix_secs(time: SystemTime) -> Result<u64> {
    Ok(time
        .duration_since(UNIX_EPOCH)
        .context("system time before Unix epoch")?
        .as_secs())
}

/// Resolves `~/.cc-profile/update-check.toml`, honoring `CC_PROFILE_UPDATE_CHECK_CACHE_PATH` in debug builds.
pub fn default_cache_path() -> Option<PathBuf> {
    #[cfg(debug_assertions)]
    if let Ok(path) = std::env::var("CC_PROFILE_UPDATE_CHECK_CACHE_PATH") {
        if !path.trim().is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    let dir = std::env::var_os("CC_PROFILE_RECEIPT_DIR")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".cc-profile")))?;
    Some(dir.join("update-check.toml"))
}

/// Returns true when passive checks are disabled via environment.
pub fn passive_checks_disabled_by_env() -> bool {
    matches!(
        std::env::var("CC_PROFILE_NO_UPDATE_CHECK").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    )
}

/// Missing cache means a passive check may run.
pub fn is_eligible_for_passive_check(
    cache: Option<&UpdateCheckCache>,
    now: SystemTime,
    min_interval: Duration,
) -> bool {
    let Some(cache) = cache else {
        return true;
    };
    let Ok(now_secs) = unix_secs(now) else {
        return true;
    };
    now_secs < cache.last_checked_unix
        || now_secs - cache.last_checked_unix >= min_interval.as_secs()
}

/// Reads cache from disk.
///
/// Missing OR unparseable (e.g. legacy RFC3339 string) cache is treated as absent, so the
/// next successful check rewrites it in the current format. This also silently masks genuine
/// corruption (e.g. a truncated or hand-edited file), which is acceptable because the cache is
/// disposable: it gets rewritten on the next successful check regardless of cause.
pub fn read_cache(path: &Path) -> Result<Option<UpdateCheckCache>> {
    if !path.is_file() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("read update check cache {}", path.display()))?;
    let cache: Option<UpdateCheckCache> = toml::from_str(&contents).ok();
    Ok(cache)
}

/// Persists cache, creating parent directories when needed.
pub fn write_cache(path: &Path, cache: &UpdateCheckCache) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create cache directory {}", parent.display()))?;
    }
    let contents = toml::to_string_pretty(cache).context("serialize update-check.toml")?;
    std::fs::write(path, contents)
        .with_context(|| format!("write update check cache {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::update_test_env_lock::{
        CcProfileNoUpdateCheckGuard, lock_cc_profile_update_check_env,
    };

    #[test]
    fn update_check_cache_no_cache_means_eligible() {
        let now = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        assert!(is_eligible_for_passive_check(
            None,
            now,
            PASSIVE_CHECK_MIN_INTERVAL
        ));
    }

    #[test]
    fn update_check_cache_recent_check_skips_lookup_eligibility() {
        let last_checked_unix = 1_700_000_000u64;
        let cache = UpdateCheckCache {
            last_checked_unix,
            latest_seen: Some("0.2.0".to_string()),
        };
        let recent = UNIX_EPOCH + Duration::from_secs(last_checked_unix + 3600);
        assert!(!is_eligible_for_passive_check(
            Some(&cache),
            recent,
            PASSIVE_CHECK_MIN_INTERVAL
        ));
    }

    #[test]
    fn update_check_cache_future_timestamp_is_eligible() {
        let now = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let cache = UpdateCheckCache {
            last_checked_unix: 1_700_000_000 + 3600,
            latest_seen: None,
        };
        assert!(is_eligible_for_passive_check(
            Some(&cache),
            now,
            PASSIVE_CHECK_MIN_INTERVAL
        ));
    }

    #[test]
    fn update_check_cache_stale_check_is_eligible() {
        let cache = UpdateCheckCache {
            last_checked_unix: 0,
            latest_seen: None,
        };
        let now = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        assert!(is_eligible_for_passive_check(
            Some(&cache),
            now,
            PASSIVE_CHECK_MIN_INTERVAL
        ));
    }

    #[test]
    fn update_check_cache_read_write_round_trip() {
        use assert_fs::TempDir;
        let temp = TempDir::new().expect("tempdir");
        let path = temp.path().join("update-check.toml");
        let cache = UpdateCheckCache {
            last_checked_unix: 1_751_328_000,
            latest_seen: Some("0.2.0".to_string()),
        };
        write_cache(&path, &cache).expect("write");
        let loaded = read_cache(&path).expect("read").expect("some");
        assert_eq!(loaded, cache);
    }

    #[test]
    fn passive_checks_disabled_when_env_set() {
        let _lock = lock_cc_profile_update_check_env();
        let _env = CcProfileNoUpdateCheckGuard::set("1");
        assert!(passive_checks_disabled_by_env());
    }

    #[test]
    fn read_cache_treats_legacy_rfc3339_string_cache_as_absent() {
        use assert_fs::TempDir;
        let temp = TempDir::new().expect("tempdir");
        let path = temp.path().join("update-check.toml");
        std::fs::write(&path, "last_checked_at = \"2026-07-01T00:00:00Z\"\n")
            .expect("write legacy cache");
        let loaded = read_cache(&path).expect("legacy cache should not error");
        assert!(loaded.is_none());
    }

    #[test]
    fn unix_secs_converts_system_time_to_whole_seconds() {
        let t = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        assert_eq!(unix_secs(t).expect("secs"), 1_700_000_000);
    }
}
