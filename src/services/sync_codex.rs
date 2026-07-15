//! Codex sync helpers: config path resolution and reserved provider-id checks.

use crate::config::model::Config;
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

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

/// Read `codex_path`, merge in every cc-profile profile, and write the result back
/// with owner-only permissions (parent dir `0o700`, file `0o600`).
///
/// An absent file is treated as an empty config (not an error). Returns the list of
/// reserved provider ids that were skipped so the caller can warn about them.
pub fn sync(config: &Config, codex_path: &Path) -> Result<Vec<String>> {
    let existing = match std::fs::read_to_string(codex_path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => {
            return Err(e)
                .with_context(|| format!("Could not read Codex config {}", codex_path.display()));
        }
    };
    let outcome = merge_codex_config(&existing, config)?;
    write_secure(codex_path, &outcome.rendered)?;
    Ok(outcome.skipped_reserved)
}

/// Write `contents` to `path`, creating the parent directory and tightening both the
/// directory (`0o700`) and file (`0o600`) to owner-only permissions.
///
/// Mirrors the permission posture in `crate::config::repository` (`0o600` / `0o700`);
/// on non-unix targets the permission tightening is a no-op, exactly as there.
fn write_secure(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "Could not create Codex config directory {}",
                parent.display()
            )
        })?;
        set_owner_only_directory_permissions(parent)?;
    }
    // Create the file with `0o600` up front so the plaintext `Bearer <api_key>` is
    // never written into a group/world-readable file, even transiently. The mode
    // only applies to a newly created file, so the `set_owner_only_permissions`
    // chmod below still runs to re-tighten a pre-existing loose file (e.g. `0o644`).
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .with_context(|| format!("Could not write Codex config {}", path.display()))?;
        file.write_all(contents.as_bytes())
            .with_context(|| format!("Could not write Codex config {}", path.display()))?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, contents)
            .with_context(|| format!("Could not write Codex config {}", path.display()))?;
    }
    set_owner_only_permissions(path)?;
    Ok(())
}

#[cfg(unix)]
fn set_owner_only_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o600);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_owner_only_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_owner_only_directory_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_owner_only_directory_permissions(_path: &Path) -> Result<()> {
    Ok(())
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

    #[test]
    fn sync_creates_file_with_merged_content_when_absent() {
        let temp = assert_fs::TempDir::new().expect("tempdir");
        let codex_path = temp.path().join(".codex").join("config.toml");
        let config = config_with(&[("profile-a", "https://a.example", "sk-a")]);

        let skipped = sync(&config, &codex_path).expect("sync should succeed");

        assert!(skipped.is_empty());
        let written = std::fs::read_to_string(&codex_path).expect("file written");
        assert!(written.contains("[model_providers.profile-a]"));
        assert!(written.contains("https://a.example"));
        assert!(written.contains("Bearer sk-a"));
    }

    #[cfg(unix)]
    #[test]
    fn sync_creates_file_with_owner_only_perms() {
        use std::os::unix::fs::PermissionsExt;
        let temp = assert_fs::TempDir::new().expect("tempdir");
        let codex_path = temp.path().join(".codex").join("config.toml");
        let config = config_with(&[("profile-a", "https://a.example", "sk-a")]);

        sync(&config, &codex_path).expect("sync should succeed");

        let mode = std::fs::metadata(&codex_path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "file perms: {mode:o}");
    }

    #[cfg(unix)]
    #[test]
    fn sync_creates_parent_dir_with_owner_only_perms() {
        use std::os::unix::fs::PermissionsExt;
        let temp = assert_fs::TempDir::new().expect("tempdir");
        let dir = temp.path().join(".codex");
        let codex_path = dir.join("config.toml");
        let config = config_with(&[("profile-a", "https://a.example", "sk-a")]);

        sync(&config, &codex_path).expect("sync should succeed");

        let mode = std::fs::metadata(&dir)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700, "dir perms: {mode:o}");
    }

    #[test]
    fn sync_preserves_existing_file_content() {
        let temp = assert_fs::TempDir::new().expect("tempdir");
        let dir = temp.path().join(".codex");
        std::fs::create_dir_all(&dir).expect("mkdir");
        let codex_path = dir.join("config.toml");
        let existing = "# hand-written\n\
model = \"gpt-5\"\n\
\n\
[model_providers.other]\n\
name = \"Other\"\n\
base_url = \"https://other.example\"\n";
        std::fs::write(&codex_path, existing).expect("write existing");
        let config = config_with(&[("profile-a", "https://a.example", "sk-a")]);

        sync(&config, &codex_path).expect("sync should succeed");

        let written = std::fs::read_to_string(&codex_path).expect("read");
        assert!(
            written.starts_with(existing),
            "existing content not preserved:\n{written}"
        );
        assert!(written.contains("[model_providers.profile-a]"));
    }

    #[cfg(unix)]
    #[test]
    fn sync_tightens_loose_existing_file_perms() {
        use std::os::unix::fs::PermissionsExt;
        let temp = assert_fs::TempDir::new().expect("tempdir");
        let dir = temp.path().join(".codex");
        std::fs::create_dir_all(&dir).expect("mkdir");
        let codex_path = dir.join("config.toml");
        std::fs::write(&codex_path, "").expect("write existing");
        std::fs::set_permissions(&codex_path, std::fs::Permissions::from_mode(0o644))
            .expect("chmod loose");
        let config = config_with(&[("profile-a", "https://a.example", "sk-a")]);

        sync(&config, &codex_path).expect("sync should succeed");

        let mode = std::fs::metadata(&codex_path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "file perms not tightened: {mode:o}");
    }

    #[cfg(unix)]
    #[test]
    fn sync_tightens_loose_existing_dir_perms() {
        use std::os::unix::fs::PermissionsExt;
        let temp = assert_fs::TempDir::new().expect("tempdir");
        let dir = temp.path().join(".codex");
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755))
            .expect("chmod loose");
        let codex_path = dir.join("config.toml");
        let config = config_with(&[("profile-a", "https://a.example", "sk-a")]);

        sync(&config, &codex_path).expect("sync should succeed");

        let mode = std::fs::metadata(&dir)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700, "dir perms not tightened: {mode:o}");
    }

    #[test]
    fn sync_returns_skipped_reserved_ids() {
        let temp = assert_fs::TempDir::new().expect("tempdir");
        let codex_path = temp.path().join(".codex").join("config.toml");
        let config = config_with(&[
            ("openai", "https://reserved.example", "sk-reserved"),
            ("profile-a", "https://a.example", "sk-a"),
        ]);

        let skipped = sync(&config, &codex_path).expect("sync should succeed");

        assert_eq!(skipped, vec!["openai".to_string()]);
        let written = std::fs::read_to_string(&codex_path).expect("read");
        assert!(!written.contains("reserved.example"));
        assert!(written.contains("[model_providers.profile-a]"));
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
