//! File-backed config repository.

use crate::config::Config;
use anyhow::{bail, Context, Result};
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
        Ok(home.join(".cc-profile"))
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
        fs::write(&self.path, contents)
            .with_context(|| format!("Could not write config file {}", self.path.display()))?;
        set_owner_only_permissions(&self.path)?;
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Profile};
    use std::collections::BTreeMap;
    use std::fs;

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
    fn load_returns_default_config_when_file_is_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repository = ConfigRepository::new(temp.path().join(".cc-profile"));

        let config = repository
            .load()
            .expect("missing file should load default config");

        assert_eq!(config, Config::default());
    }

    #[test]
    fn save_then_load_round_trips_toml_config() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repository = ConfigRepository::new(temp.path().join(".cc-profile"));
        let config = sample_config();

        repository.save(&config).expect("save should succeed");
        let loaded = repository.load().expect("load should succeed");

        assert_eq!(loaded, config);
    }

    #[test]
    fn load_rejects_newer_config_version_without_overwriting_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join(".cc-profile");
        fs::write(&path, "version = 999\n").expect("write config");
        let repository = ConfigRepository::new(path.clone());

        let error = repository.load().expect_err("newer version should fail");

        assert!(error.to_string().contains("Unsupported config version 999"));
        assert_eq!(
            fs::read_to_string(path).expect("read config"),
            "version = 999\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn existing_broad_permissions_are_reported_and_can_be_fixed() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join(".cc-profile");
        fs::write(&path, "version = 1\n").expect("write config");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644))
            .expect("set broad permissions");
        let repository = ConfigRepository::new(path.clone());

        assert!(repository
            .has_broad_permissions()
            .expect("check permissions"));
        repository.fix_permissions().expect("fix permissions");

        let mode = fs::metadata(path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
