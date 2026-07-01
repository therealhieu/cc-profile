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
    pub last_checked_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_seen: Option<String>,
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
    let Ok(last) = parse_rfc3339_utc(&cache.last_checked_at) else {
        return true;
    };
    now.duration_since(last)
        .map(|elapsed| elapsed >= min_interval)
        .unwrap_or(true)
}

/// Reads cache from disk; missing file is `Ok(None)`.
pub fn read_cache(path: &Path) -> Result<Option<UpdateCheckCache>> {
    if !path.is_file() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("read update check cache {}", path.display()))?;
    let cache: UpdateCheckCache = toml::from_str(&contents).context("parse update-check.toml")?;
    Ok(Some(cache))
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

/// Formats `time` as RFC3339 UTC (`…Z`).
pub fn format_rfc3339_utc(time: SystemTime) -> Result<String> {
    let duration = time
        .duration_since(UNIX_EPOCH)
        .context("system time before Unix epoch")?;
    let secs = duration.as_secs();
    let nanos = duration.subsec_nanos();
    Ok(format_utc_from_unix(secs, nanos))
}

fn format_utc_from_unix(secs: u64, nanos: u32) -> String {
    let days = (secs / 86_400) as i32;
    let time_of_day = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = time_of_day / 3600;
    let minute = (time_of_day % 3600) / 60;
    let second = time_of_day % 60;
    if nanos == 0 {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
    } else {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{nanos:09}Z")
    }
}

/// Parses RFC3339 UTC timestamps used in the cache (`YYYY-MM-DDTHH:MM:SSZ` or with fractional seconds).
pub fn parse_rfc3339_utc(s: &str) -> Result<SystemTime> {
    let s = s.trim();
    let (date_time, frac_nanos) = match s.split_once('.') {
        Some((base, rest)) if rest.ends_with('Z') => {
            let frac = rest.trim_end_matches('Z');
            let nanos: u32 = if frac.is_empty() {
                0
            } else {
                let padded = format!("{frac:0<9}");
                padded[..9]
                    .parse()
                    .context("invalid fractional seconds in last_checked_at")?
            };
            (base, nanos)
        }
        _ => (s.trim_end_matches('Z'), 0),
    };
    if !s.ends_with('Z') {
        anyhow::bail!("last_checked_at must be UTC (Z suffix)");
    }
    let mut parts = date_time.split('T');
    let date = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing date in last_checked_at"))?;
    let time = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing time in last_checked_at"))?;
    if parts.next().is_some() {
        anyhow::bail!("invalid last_checked_at format");
    }
    let mut date_parts = date.split('-');
    let year: i32 = date_parts.next().context("year")?.parse().context("year")?;
    let month: u32 = date_parts
        .next()
        .context("month")?
        .parse()
        .context("month")?;
    let day: u32 = date_parts.next().context("day")?.parse().context("day")?;
    if date_parts.next().is_some() {
        anyhow::bail!("invalid date in last_checked_at");
    }
    let mut time_parts = time.split(':');
    let hour: u32 = time_parts.next().context("hour")?.parse().context("hour")?;
    let minute: u32 = time_parts
        .next()
        .context("minute")?
        .parse()
        .context("minute")?;
    let second: u32 = time_parts
        .next()
        .context("second")?
        .parse()
        .context("second")?;
    if time_parts.next().is_some() {
        anyhow::bail!("invalid time in last_checked_at");
    }
    let days = days_from_civil(year, month, day)?;
    let secs =
        days as u64 * 86_400 + u64::from(hour) * 3600 + u64::from(minute) * 60 + u64::from(second);
    Ok(UNIX_EPOCH + Duration::new(secs, frac_nanos))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> Result<i32> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        anyhow::bail!("invalid civil date");
    }
    let mut y = year;
    y -= if month <= 2 { 1 } else { 0 };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let month = month as i32;
    let day = day as i32;
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Ok(era * 146_097 + doe - 719_468)
}

fn civil_from_days(z: i32) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn env_lock() -> MutexGuard<'static, ()> {
        ENV_LOCK.lock().expect("env lock")
    }

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
        let cache = UpdateCheckCache {
            last_checked_at: "2026-07-01T00:00:00Z".to_string(),
            latest_seen: Some("0.2.0".to_string()),
        };
        let last = parse_rfc3339_utc(&cache.last_checked_at).expect("parse");
        let recent = last + Duration::from_secs(3600);
        assert!(!is_eligible_for_passive_check(
            Some(&cache),
            recent,
            PASSIVE_CHECK_MIN_INTERVAL
        ));
    }

    #[test]
    fn update_check_cache_stale_check_is_eligible() {
        let cache = UpdateCheckCache {
            last_checked_at: "2020-01-01T00:00:00Z".to_string(),
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
            last_checked_at: "2026-07-01T00:00:00Z".to_string(),
            latest_seen: Some("0.2.0".to_string()),
        };
        write_cache(&path, &cache).expect("write");
        let loaded = read_cache(&path).expect("read").expect("some");
        assert_eq!(loaded, cache);
    }

    #[test]
    fn passive_checks_disabled_when_env_set() {
        let _guard = env_lock();
        // SAFETY: serialized by ENV_LOCK in this test module.
        unsafe {
            std::env::set_var("CC_PROFILE_NO_UPDATE_CHECK", "1");
        }
        assert!(passive_checks_disabled_by_env());
        unsafe {
            std::env::remove_var("CC_PROFILE_NO_UPDATE_CHECK");
        }
    }

    #[test]
    fn format_and_parse_rfc3339_utc_round_trip() {
        let t = UNIX_EPOCH + Duration::new(1_700_000_000, 0);
        let s = format_rfc3339_utc(t).expect("format");
        let parsed = parse_rfc3339_utc(&s).expect("parse");
        assert_eq!(parsed, t);
    }
}
