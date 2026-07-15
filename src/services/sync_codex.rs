//! Codex sync helpers: config path resolution and reserved provider-id checks.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Codex built-in provider ids that a synced profile must never overwrite.
// Consumed by the merge core (Task 1.3); only tests reference it in this task.
#[allow(dead_code)]
const RESERVED_PROVIDER_IDS: [&str; 3] = ["openai", "ollama", "lmstudio"];

// Consumed by the merge core (Task 1.3); only tests reference it in this task.
#[allow(dead_code)]
pub(crate) fn is_reserved_provider_id(name: &str) -> bool {
    RESERVED_PROVIDER_IDS.contains(&name)
}

/// `$CODEX_HOME/config.toml` if set, else `~/.codex/config.toml`.
pub fn codex_config_path() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("CODEX_HOME") {
        return Ok(PathBuf::from(dir).join("config.toml"));
    }
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".codex").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serializes tests that read or mutate the process-global `CODEX_HOME` env var.
    ///
    /// `cargo test` runs tests as parallel threads in one process, so a test that
    /// sets/removes `CODEX_HOME` would race the home-fallback test without this lock.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Restores `CODEX_HOME` when dropped (including on panic).
    struct CodexHomeGuard {
        previous: Option<std::ffi::OsString>,
    }

    impl CodexHomeGuard {
        fn set(value: &std::path::Path) -> Self {
            let previous = std::env::var_os("CODEX_HOME");
            // SAFETY: caller holds `ENV_LOCK`.
            unsafe {
                std::env::set_var("CODEX_HOME", value);
            }
            Self { previous }
        }

        fn clear() -> Self {
            let previous = std::env::var_os("CODEX_HOME");
            // SAFETY: caller holds `ENV_LOCK`.
            unsafe {
                std::env::remove_var("CODEX_HOME");
            }
            Self { previous }
        }
    }

    impl Drop for CodexHomeGuard {
        fn drop(&mut self) {
            // SAFETY: only used while `ENV_LOCK` is held.
            unsafe {
                match &self.previous {
                    Some(value) => std::env::set_var("CODEX_HOME", value),
                    None => std::env::remove_var("CODEX_HOME"),
                }
            }
        }
    }

    #[test]
    fn is_reserved_provider_id_true_for_codex_builtins() {
        assert!(is_reserved_provider_id("openai"));
        assert!(is_reserved_provider_id("ollama"));
        assert!(is_reserved_provider_id("lmstudio"));
    }

    #[test]
    fn is_reserved_provider_id_false_for_other_ids() {
        assert!(!is_reserved_provider_id("profile-a"));
        assert!(!is_reserved_provider_id("anthropic"));
        assert!(!is_reserved_provider_id(""));
    }

    #[test]
    fn codex_config_path_uses_codex_home_without_appending_dot_codex() {
        let _lock = ENV_LOCK.lock().expect("env lock poisoned");
        let temp = assert_fs::TempDir::new().expect("tempdir");
        let _guard = CodexHomeGuard::set(temp.path());

        assert_eq!(
            codex_config_path().expect("path"),
            temp.path().join("config.toml")
        );
    }

    #[test]
    fn codex_config_path_falls_back_to_home_dot_codex_when_unset() {
        let _lock = ENV_LOCK.lock().expect("env lock poisoned");
        let _guard = CodexHomeGuard::clear();
        let home = dirs::home_dir().expect("home dir");

        assert_eq!(
            codex_config_path().expect("path"),
            home.join(".codex").join("config.toml")
        );

        // `CODEX_HOME` must remain unset for other tests relying on its absence.
        assert!(std::env::var_os("CODEX_HOME").is_none());
    }
}
