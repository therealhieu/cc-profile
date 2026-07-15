//! Codex sync helpers: config path resolution and reserved provider-id checks.

use crate::config::model::Config;
use anyhow::{Context, Result, bail};
use std::path::PathBuf;

/// Codex built-in provider ids that a synced profile must never overwrite.
const RESERVED_PROVIDER_IDS: [&str; 3] = ["openai", "ollama", "lmstudio"];

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

/// Result of merging cc-profile profiles into Codex TOML text.
// Consumed by the sync entry point (Task 2.1); only tests reference it in this task.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct MergeOutcome {
    pub rendered: String,
    pub skipped_reserved: Vec<String>,
}

/// Merge cc-profile profiles into existing Codex TOML text (empty string = new file).
///
/// Overwrites exactly three managed keys on each `[model_providers.<name>]` table
/// (`name`, `base_url`, `http_headers`) and leaves every other table, key, and
/// comment untouched. Reserved provider ids are skipped and reported to the caller.
// Consumed by the sync entry point (Task 2.1); only tests reference it in this task.
#[allow(dead_code)]
pub(crate) fn merge_codex_config(existing: &str, config: &Config) -> Result<MergeOutcome> {
    let mut doc: toml_edit::DocumentMut =
        existing.parse().context("Invalid TOML in Codex config")?;

    // Guard: a pre-existing non-table `model_providers` must Err, not panic on index.
    // Written as a nested `if let` (not a let-chain) to compile on the declared MSRV
    // 1.85; let-chains only stabilized in Rust 1.88.
    if let Some(item) = doc.get("model_providers") {
        if !item.is_table_like() {
            bail!("Codex config `model_providers` is not a table");
        }
    }

    let mut skipped = Vec::new();
    for (name, profile) in &config.profiles {
        if is_reserved_provider_id(name) {
            skipped.push(name.clone());
            continue;
        }

        // `or_insert(table())` forces a standard `[model_providers.<name>]` block
        // (auto-vivified indexing would render an inline table) while keeping any
        // pre-existing table and its hand-added keys intact.
        let providers = doc["model_providers"].or_insert(toml_edit::table());
        let table = providers[name.as_str()].or_insert(toml_edit::table());
        // If the provider key already exists as a scalar (e.g. `profile-a = "x"`),
        // `or_insert` is a no-op and `table` is not table-like; indexing it below would
        // panic. Bail with a clear message instead.
        if !table.is_table_like() {
            bail!("Codex config `model_providers.{name}` is not a table");
        }
        // Overwrite only these three managed keys; any hand-added sub-key is preserved.
        table["name"] = toml_edit::value(name.as_str());
        table["base_url"] = toml_edit::value(profile.endpoint.as_str());

        // http_headers is an inline table: { Authorization = "Bearer <key>" }
        let mut headers = toml_edit::InlineTable::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", profile.api_key).into(),
        );
        table["http_headers"] = toml_edit::value(headers);
    }

    Ok(MergeOutcome {
        rendered: doc.to_string(),
        skipped_reserved: skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{Config, Profile};
    use std::sync::Mutex;

    /// Builds a minimal `Config` with the given `(name, endpoint, api_key)` profiles.
    fn config_with(profiles: &[(&str, &str, &str)]) -> Config {
        let mut config = Config::default();
        for (name, endpoint, api_key) in profiles {
            config.profiles.insert(
                (*name).to_string(),
                Profile::builder()
                    .endpoint((*endpoint).to_string())
                    .api_key((*api_key).to_string())
                    .fable("fable".to_string())
                    .opus("opus".to_string())
                    .sonnet("sonnet".to_string())
                    .haiku("haiku".to_string())
                    .build(),
            );
        }
        config
    }

    #[test]
    fn merge_into_empty_writes_provider_block_with_managed_keys() {
        let config = config_with(&[("profile-a", "https://a.example", "sk-a")]);

        let outcome = merge_codex_config("", &config).expect("merge should succeed");
        let doc: toml_edit::DocumentMut = outcome.rendered.parse().expect("valid TOML");
        let table = doc["model_providers"]["profile-a"]
            .as_table_like()
            .expect("provider is a table");

        assert_eq!(
            table.get("name").and_then(|v| v.as_str()),
            Some("profile-a")
        );
        assert_eq!(
            table.get("base_url").and_then(|v| v.as_str()),
            Some("https://a.example")
        );
        let headers = table
            .get("http_headers")
            .and_then(|v| v.as_inline_table())
            .expect("http_headers inline table");
        assert_eq!(
            headers.get("Authorization").and_then(|v| v.as_str()),
            Some("Bearer sk-a")
        );
        // Exactly the three managed keys — no leaked `wire_api`/`model`/etc.
        assert_eq!(table.iter().count(), 3);
        assert!(outcome.skipped_reserved.is_empty());
    }

    #[test]
    fn merge_preserves_foreign_table_top_level_key_and_comment() {
        let existing = r#"# hand-written config
model = "gpt-5"

[model_providers.other]
name = "Other"
base_url = "https://other.example"
"#;
        let config = config_with(&[("profile-a", "https://a.example", "sk-a")]);

        let rendered = merge_codex_config(existing, &config)
            .expect("merge should succeed")
            .rendered;

        assert!(rendered.contains("# hand-written config"));
        assert!(rendered.contains("model = \"gpt-5\""));
        assert!(rendered.contains("[model_providers.other]"));
        assert!(rendered.contains("https://other.example"));
        // The new provider was added without disturbing the foreign one.
        assert!(rendered.contains("[model_providers.profile-a]"));
    }

    /// Byte-for-byte proof that merging preserves the entire preamble/foreign region
    /// (comment, top-level key, blank line, foreign provider block) verbatim — the
    /// reason `toml_edit` was chosen over `toml`. `.contains()` only proves substrings
    /// appear; this asserts the exact original text survives as a prefix, so layout,
    /// comments, key order, and whitespace before the insertion point are undisturbed.
    #[test]
    fn merge_preserves_existing_region_byte_for_byte() {
        let existing = "# hand-written config\n\
model = \"gpt-5\"\n\
\n\
[model_providers.other]\n\
name = \"Other\"\n\
base_url = \"https://other.example\"\n";
        let config = config_with(&[("profile-a", "https://a.example", "sk-a")]);

        let rendered = merge_codex_config(existing, &config)
            .expect("merge should succeed")
            .rendered;

        // The original text must be reproduced verbatim at the start of the output;
        // the new provider block is appended after it.
        assert!(
            rendered.starts_with(existing),
            "existing region not preserved byte-for-byte.\n--- expected prefix ---\n{existing}\n--- actual output ---\n{rendered}"
        );
    }

    #[test]
    fn merge_overwrites_managed_keys_but_preserves_hand_added_keys() {
        let existing = r#"[model_providers.profile-a]
name = "stale"
base_url = "https://stale.example"
wire_api = "chat"

[model_providers.profile-a.http_headers]
Authorization = "Bearer stale"
"#;
        let config = config_with(&[("profile-a", "https://fresh.example", "sk-fresh")]);

        let rendered = merge_codex_config(existing, &config)
            .expect("merge should succeed")
            .rendered;
        let doc: toml_edit::DocumentMut = rendered.parse().expect("valid TOML");
        let table = doc["model_providers"]["profile-a"]
            .as_table_like()
            .expect("provider is a table");

        // Managed keys overwritten.
        assert_eq!(
            table.get("name").and_then(|v| v.as_str()),
            Some("profile-a")
        );
        assert_eq!(
            table.get("base_url").and_then(|v| v.as_str()),
            Some("https://fresh.example")
        );
        assert!(rendered.contains("Bearer sk-fresh"));
        assert!(!rendered.contains("Bearer stale"));
        // Hand-added key survives verbatim.
        assert_eq!(table.get("wire_api").and_then(|v| v.as_str()), Some("chat"));
    }

    #[test]
    fn merge_skips_reserved_provider_ids() {
        let config = config_with(&[
            ("openai", "https://reserved.example", "sk-reserved"),
            ("profile-a", "https://a.example", "sk-a"),
        ]);

        let outcome = merge_codex_config("", &config).expect("merge should succeed");

        assert_eq!(outcome.skipped_reserved, vec!["openai".to_string()]);
        assert!(!outcome.rendered.contains("reserved.example"));
        assert!(outcome.rendered.contains("[model_providers.profile-a]"));
    }

    #[test]
    fn merge_is_idempotent() {
        let config = config_with(&[
            ("profile-a", "https://a.example", "sk-a"),
            ("profile-b", "https://b.example", "sk-b"),
        ]);

        let once = merge_codex_config("", &config)
            .expect("first merge")
            .rendered;
        let twice = merge_codex_config(&once, &config)
            .expect("second merge")
            .rendered;

        assert_eq!(once, twice);
    }

    #[test]
    fn merge_rejects_invalid_toml() {
        let config = config_with(&[("profile-a", "https://a.example", "sk-a")]);

        let err = merge_codex_config("this is = = not toml", &config)
            .expect_err("invalid TOML should error");

        assert!(
            err.to_string().contains("Invalid TOML in Codex config"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn merge_rejects_non_table_model_providers() {
        let config = config_with(&[("profile-a", "https://a.example", "sk-a")]);

        let err = merge_codex_config("model_providers = \"x\"", &config)
            .expect_err("non-table model_providers should error");

        assert!(
            err.to_string().contains("model_providers"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn merge_rejects_scalar_provider_colliding_with_profile() {
        // `model_providers` is a table, but the `profile-a` key inside it is a
        // scalar string. `or_insert` is then a no-op, so writing managed keys must
        // `bail!` rather than panic via `IndexMut`.
        let existing = "[model_providers]\nprofile-a = \"x\"\n";
        let config = config_with(&[("profile-a", "https://a.example", "sk-a")]);

        let err = merge_codex_config(existing, &config)
            .expect_err("scalar provider collision should error, not panic");

        assert!(
            err.to_string().contains("model_providers.profile-a"),
            "unexpected error: {err}"
        );
    }

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
