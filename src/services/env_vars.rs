//! Global environment variable mutations on [`Config::envs`].
//!
//! Keys are validated before any change; values are stored as provided (no required-value check).

use crate::config::validation::validate_env_key;
use crate::config::Config;
use anyhow::Result;

/// Inserts or updates a global environment variable on `config`.
///
/// # Errors
///
/// Returns an error when `key` fails [`validate_env_key`].
pub fn set_env_var(config: &mut Config, key: &str, value: &str) -> Result<()> {
    validate_env_key(key)?;
    config.envs.insert(key.to_string(), value.to_string());
    Ok(())
}

/// Removes a global environment variable from `config` when present.
///
/// Missing keys are a no-op; the underlying map returns `None`.
///
/// # Errors
///
/// Returns an error when `key` fails [`validate_env_key`].
pub fn delete_env_var(config: &mut Config, key: &str) -> Result<()> {
    validate_env_key(key)?;
    config.envs.remove(key);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_env_var_adds_or_updates_global_env_value() {
        let mut config = Config::default();

        set_env_var(&mut config, "HTTP_PROXY", "http://localhost:7890").expect("set should work");
        assert_eq!(
            config.envs.get("HTTP_PROXY").map(String::as_str),
            Some("http://localhost:7890")
        );

        set_env_var(&mut config, "HTTP_PROXY", "http://localhost:8888")
            .expect("update should work");

        assert_eq!(
            config.envs.get("HTTP_PROXY").map(String::as_str),
            Some("http://localhost:8888")
        );
    }

    #[test]
    fn delete_env_var_removes_existing_global_env_value() {
        let mut config = Config::default();
        set_env_var(&mut config, "CUSTOM_FLAG", "enabled").expect("set should work");

        delete_env_var(&mut config, "CUSTOM_FLAG").expect("delete should work");

        assert!(!config.envs.contains_key("CUSTOM_FLAG"));
    }

    #[test]
    fn set_env_var_rejects_invalid_env_key() {
        let mut config = Config::default();

        let err = set_env_var(&mut config, "http_proxy", "value").unwrap_err();
        assert!(err.to_string().contains("A-Z or underscore"));
        assert!(!config.envs.contains_key("http_proxy"));
    }

    #[test]
    fn delete_env_var_rejects_invalid_env_key() {
        let mut config = Config::default();
        config
            .envs
            .insert("HTTP_PROXY".to_string(), "x".to_string());

        let err = delete_env_var(&mut config, "1FLAG").unwrap_err();
        assert!(err.to_string().contains("A-Z or underscore"));
        assert_eq!(config.envs.get("HTTP_PROXY").map(String::as_str), Some("x"));
    }
}
