//! File-backed config repository.

use crate::config::Config;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

const CURRENT_CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct ConfigRepository {
    path: PathBuf,
}

impl ConfigRepository {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn default_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        Ok(home.join(".cc-profile").join("config.toml"))
    }

    /// Fallible default path resolution; plan and later callers use `ConfigRepository::default()?`.
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Result<Self> {
        Ok(Self::new(Self::default_path()?))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<Config> {
        if !self.path.exists() {
            return Ok(Config::default());
        }

        let contents = fs::read_to_string(&self.path)
            .with_context(|| format!("Could not read config file {}", self.path.display()))?;
        let config: Config = toml::from_str(&contents)
            .with_context(|| format!("Invalid TOML in config file {}", self.path.display()))?;

        if config.version > CURRENT_CONFIG_VERSION {
            bail!("Unsupported config version {}", config.version);
        }

        Ok(config)
    }

    pub fn has_broad_permissions(&self) -> Result<bool> {
        has_broad_permissions(&self.path)
    }

    pub fn fix_permissions(&self) -> Result<()> {
        set_owner_only_permissions(&self.path)
    }

    pub fn save(&self, config: &Config) -> Result<()> {
        let contents = toml::to_string_pretty(config).context("Could not serialize config")?;
        ensure_config_parent_directory(&self.path)?;
        fs::write(&self.path, contents)
            .with_context(|| format!("Could not write config file {}", self.path.display()))?;
        set_owner_only_permissions(&self.path)?;
        Ok(())
    }

    /// Loads the config, applies `mutate`, persists the result, and returns the saved config.
    ///
    /// # Errors
    ///
    /// Returns an error when the config cannot be loaded, `mutate` returns an error
    /// (in which case nothing is written), or the result cannot be saved.
    pub fn update(&self, mutate: impl FnOnce(&mut Config) -> Result<()>) -> Result<Config> {
        let mut config = self.load()?;
        mutate(&mut config)?;
        self.save(&config)?;
        Ok(config)
    }
}

fn ensure_config_parent_directory(config_path: &Path) -> Result<()> {
    let Some(parent) = config_path.parent() else {
        bail!(
            "Invalid config path {}: missing parent directory",
            config_path.display()
        );
    };

    if parent.is_file() {
        bail!(
            "Cannot create config at {} because {} is a file, not a directory. \
             Move or remove that file so cc-profile can use {} as a directory.",
            config_path.display(),
            parent.display(),
            parent.display()
        );
    }

    fs::create_dir_all(parent)
        .with_context(|| format!("Could not create config directory {}", parent.display()))?;
    set_owner_only_directory_permissions(parent)?;
    Ok(())
}

#[cfg(unix)]
fn has_broad_permissions(path: &Path) -> Result<bool> {
    use std::os::unix::fs::PermissionsExt;

    if !path.exists() {
        return Ok(false);
    }

    let mode = fs::metadata(path)?.permissions().mode() & 0o777;
    Ok(mode & 0o077 != 0)
}

#[cfg(not(unix))]
fn has_broad_permissions(_path: &Path) -> Result<bool> {
    Ok(false)
}

#[cfg(unix)]
fn set_owner_only_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o600);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_owner_only_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_owner_only_directory_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_owner_only_directory_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Profile};
    use std::collections::BTreeMap;
    use std::fs;

    fn cc_profile_dir(root: &Path) -> PathBuf {
        root.join(".cc-profile")
    }

    fn sample_config() -> Config {
        Config {
            active_profile: Some("profile-a".to_string()),
            profiles: BTreeMap::from([(
                "profile-a".to_string(),
                Profile::builder()
                    .endpoint("https://api.anthropic.com".to_string())
                    .api_key("sk-ant-secret".to_string())
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
    fn default_path_appends_cc_profile_config_toml_under_home() {
        let home = dirs::home_dir().expect("home dir");
        let path = ConfigRepository::default_path().expect("default path");
        assert_eq!(path, home.join(".cc-profile").join("config.toml"));
    }

    #[test]
    fn save_creates_config_directory_before_writing_config_toml() {
        let temp = tempfile::tempdir().expect("tempdir");
        let config_path = cc_profile_dir(temp.path()).join("config.toml");
        let repository = ConfigRepository::new(config_path.clone());
        let config = sample_config();

        repository.save(&config).expect("save should create layout");

        assert!(config_path.is_file());
        assert_eq!(repository.load().expect("load after save"), config);
    }

    #[cfg(unix)]
    #[test]
    fn save_tightens_existing_config_directory_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().expect("tempdir");
        let dir = cc_profile_dir(temp.path());
        fs::create_dir_all(&dir).expect("create dir");
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).expect("set 0755");
        let config_path = dir.join("config.toml");
        let repository = ConfigRepository::new(config_path);

        repository
            .save(&Config::default())
            .expect("save should tighten dir perms");

        let mode = fs::metadata(&dir).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);
    }

    #[test]
    fn save_errors_when_cc_profile_path_is_existing_file_and_preserves_contents() {
        let temp = tempfile::tempdir().expect("tempdir");
        let legacy = cc_profile_dir(temp.path());
        let legacy_contents = "version = 1\nactive_profile = \"legacy\"\n";
        fs::write(&legacy, legacy_contents).expect("write legacy file");
        let config_path = legacy.join("config.toml");
        let repository = ConfigRepository::new(config_path);

        let error = repository
            .save(&Config::default())
            .expect_err("save must fail when .cc-profile is a file");

        let message = error.to_string();
        assert!(
            message.contains(".cc-profile"),
            "error should mention .cc-profile, got: {message}"
        );
        assert_eq!(
            fs::read_to_string(&legacy).expect("legacy file unchanged"),
            legacy_contents
        );
    }

    #[test]
    fn load_returns_default_config_when_file_is_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repository = ConfigRepository::new(temp.path().join("plain-config.toml"));

        let config = repository
            .load()
            .expect("missing file should load default config");

        assert_eq!(config, Config::default());
    }

    #[test]
    fn save_then_load_round_trips_toml_config() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repository = ConfigRepository::new(temp.path().join("plain-config.toml"));
        let config = sample_config();

        repository.save(&config).expect("save should succeed");
        let loaded = repository.load().expect("load should succeed");

        assert_eq!(loaded, config);
    }

    #[test]
    fn load_rejects_newer_config_version_without_overwriting_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("plain-config.toml");
        fs::write(&path, "version = 999\n").expect("write config");
        let repository = ConfigRepository::new(path.clone());

        let error = repository.load().expect_err("newer version should fail");

        assert!(error.to_string().contains("Unsupported config version 999"));
        assert_eq!(
            fs::read_to_string(path).expect("read config"),
            "version = 999\n"
        );
    }

    #[test]
    fn update_persists_mutation_and_returns_saved_config() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repository = ConfigRepository::new(temp.path().join("plain-config.toml"));
        repository
            .save(&sample_config())
            .expect("seed save should succeed");

        let updated = repository
            .update(|config| {
                config.active_profile = Some("profile-b".to_string());
                Ok(())
            })
            .expect("update should succeed");

        assert_eq!(updated.active_profile, Some("profile-b".to_string()));
        let reloaded = repository.load().expect("load after update");
        assert_eq!(reloaded, updated);
        assert_eq!(reloaded.active_profile, Some("profile-b".to_string()));
    }

    #[test]
    fn update_propagates_mutator_error_without_writing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("plain-config.toml");
        let repository = ConfigRepository::new(path.clone());
        let seed = sample_config();
        repository.save(&seed).expect("seed save should succeed");
        let before = fs::read_to_string(&path).expect("read seeded config");

        let error = repository
            .update(|_config| Err(anyhow::anyhow!("boom")))
            .expect_err("mutator error should propagate");

        assert!(error.to_string().contains("boom"));
        let after = fs::read_to_string(&path).expect("read config after failed update");
        assert_eq!(after, before, "file must be unchanged when mutator fails");
    }

    #[cfg(unix)]
    #[test]
    fn existing_broad_permissions_are_reported_and_can_be_fixed() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("plain-config.toml");
        fs::write(&path, "version = 1\n").expect("write config");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644))
            .expect("set broad permissions");
        let repository = ConfigRepository::new(path.clone());

        assert!(
            repository
                .has_broad_permissions()
                .expect("check permissions")
        );
        repository.fix_permissions().expect("fix permissions");

        let mode = fs::metadata(path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
