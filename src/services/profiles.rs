//! Profile lifecycle operations on in-memory [`Config`] state.

use crate::config::validation::{validate_profile_name, validate_required_value};
use crate::config::{Config, Profile};
use anyhow::{bail, Result};

/// Creates a named profile and optionally sets it as active.
///
/// # Errors
///
/// Returns an error when the name is invalid, the profile fields fail validation, or a profile
/// with the same name already exists.
pub fn create_profile(
    config: &mut Config,
    name: &str,
    profile: Profile,
    set_active: bool,
) -> Result<()> {
    validate_profile_name(name)?;
    validate_profile(&profile)?;

    if config.profiles.contains_key(name) {
        bail!("Profile '{name}' already exists");
    }

    config.profiles.insert(name.to_string(), profile);
    if set_active {
        config.active_profile = Some(name.to_string());
    }
    Ok(())
}

/// Sets the active profile to an existing name.
///
/// # Errors
///
/// Returns an error when no profile with `name` exists.
pub fn set_active_profile(config: &mut Config, name: &str) -> Result<()> {
    if !config.profiles.contains_key(name) {
        bail!("Profile '{name}' does not exist");
    }
    config.active_profile = Some(name.to_string());
    Ok(())
}

/// Renames a profile and updates [`Config::active_profile`] when it pointed at the old name.
///
/// # Errors
///
/// Returns an error when `new_name` is invalid, `new_name` is already taken, or `old_name` does
/// not exist.
pub fn rename_profile(config: &mut Config, old_name: &str, new_name: &str) -> Result<()> {
    validate_profile_name(new_name)?;
    if config.profiles.contains_key(new_name) {
        bail!("Profile '{new_name}' already exists");
    }
    let profile = config
        .profiles
        .remove(old_name)
        .ok_or_else(|| anyhow::anyhow!("Profile '{old_name}' does not exist"))?;
    config.profiles.insert(new_name.to_string(), profile);
    if config.active_profile.as_deref() == Some(old_name) {
        config.active_profile = Some(new_name.to_string());
    }
    Ok(())
}

/// Replaces the stored profile for `name` with `profile`.
///
/// # Errors
///
/// Returns an error when `name` does not exist or `profile` fails validation.
pub fn update_profile(config: &mut Config, name: &str, profile: Profile) -> Result<()> {
    if !config.profiles.contains_key(name) {
        bail!("Profile '{name}' does not exist");
    }
    validate_profile(&profile)?;
    let slot = config
        .profiles
        .get_mut(name)
        .expect("profile exists after contains_key check");
    *slot = profile;
    Ok(())
}

/// Removes a profile and clears active when it was the active profile.
///
/// # Errors
///
/// Returns an error when `name` does not exist.
pub fn delete_profile(config: &mut Config, name: &str) -> Result<()> {
    config
        .profiles
        .remove(name)
        .ok_or_else(|| anyhow::anyhow!("Profile '{name}' does not exist"))?;
    if config.active_profile.as_deref() == Some(name) {
        config.active_profile = None;
    }
    Ok(())
}

fn validate_profile(profile: &Profile) -> Result<()> {
    validate_required_value("Endpoint", &profile.endpoint)?;
    validate_required_value("API key", &profile.api_key)?;
    validate_required_value("Fable model", &profile.fable)?;
    validate_required_value("Opus model", &profile.opus)?;
    validate_required_value("Sonnet model", &profile.sonnet)?;
    validate_required_value("Haiku model", &profile.haiku)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(endpoint: &str) -> Profile {
        Profile::builder()
            .endpoint(endpoint.to_string())
            .api_key("sk-ant-secret".to_string())
            .fable("claude-fable-5".to_string())
            .opus("claude-opus-4-8".to_string())
            .sonnet("claude-sonnet-4-6".to_string())
            .haiku("claude-haiku-4-5-20251001".to_string())
            .build()
    }

    #[test]
    fn create_profile_saves_profile_and_sets_active_when_requested() {
        let mut config = Config::default();

        create_profile(
            &mut config,
            "profile-a",
            profile("https://api.anthropic.com"),
            true,
        )
        .expect("profile should be created");

        assert_eq!(config.active_profile.as_deref(), Some("profile-a"));
        assert!(config.profiles.contains_key("profile-a"));
    }

    #[test]
    fn rename_profile_updates_active_profile_when_profile_was_active() {
        let mut config = Config::default();
        create_profile(
            &mut config,
            "profile-a",
            profile("https://api.anthropic.com"),
            true,
        )
        .expect("profile should be created");

        rename_profile(&mut config, "profile-a", "profile-b").expect("rename should succeed");

        assert_eq!(config.active_profile.as_deref(), Some("profile-b"));
        assert!(!config.profiles.contains_key("profile-a"));
        assert!(config.profiles.contains_key("profile-b"));
    }

    #[test]
    fn delete_active_profile_clears_active_profile() {
        let mut config = Config::default();
        create_profile(
            &mut config,
            "profile-a",
            profile("https://api.anthropic.com"),
            true,
        )
        .expect("profile should be created");

        delete_profile(&mut config, "profile-a").expect("delete should succeed");

        assert!(config.active_profile.is_none());
        assert!(config.profiles.is_empty());
    }

    fn assert_err_contains(result: Result<()>, needle: &str) {
        let err = result.expect_err("expected error");
        let msg = err.to_string();
        assert!(
            msg.contains(needle),
            "expected error containing {needle:?}, got {msg:?}"
        );
    }

    #[test]
    fn create_profile_errors_when_profile_already_exists() {
        let mut config = Config::default();
        create_profile(
            &mut config,
            "profile-a",
            profile("https://api.anthropic.com"),
            false,
        )
        .expect("first create should succeed");

        let err = create_profile(
            &mut config,
            "profile-a",
            profile("https://api.anthropic.com"),
            false,
        );
        assert_err_contains(err, "Profile 'profile-a' already exists");
    }

    #[test]
    fn create_profile_errors_on_invalid_profile_name() {
        let mut config = Config::default();
        let err = create_profile(
            &mut config,
            "bad name",
            profile("https://api.anthropic.com"),
            false,
        );
        assert_err_contains(
            err,
            "Profile name must contain only letters, numbers, dashes, and underscores",
        );
    }

    #[test]
    fn create_profile_errors_when_required_profile_field_is_blank() {
        let mut config = Config::default();
        let blank_endpoint = Profile::builder()
            .endpoint("   ".to_string())
            .api_key("sk-ant-secret".to_string())
            .fable("claude-fable-5".to_string())
            .opus("claude-opus-4-8".to_string())
            .sonnet("claude-sonnet-4-6".to_string())
            .haiku("claude-haiku-4-5-20251001".to_string())
            .build();
        let err = create_profile(&mut config, "profile-a", blank_endpoint, false);
        assert_err_contains(err, "Endpoint must not be empty");
    }

    #[test]
    fn set_active_profile_errors_when_profile_missing() {
        let mut config = Config::default();
        let err = set_active_profile(&mut config, "missing");
        assert_err_contains(err, "Profile 'missing' does not exist");
    }

    #[test]
    fn rename_profile_errors_when_source_missing() {
        let mut config = Config::default();
        let err = rename_profile(&mut config, "missing", "profile-b");
        assert_err_contains(err, "Profile 'missing' does not exist");
    }

    #[test]
    fn rename_profile_errors_when_target_name_already_exists() {
        let mut config = Config::default();
        create_profile(
            &mut config,
            "profile-a",
            profile("https://api.anthropic.com"),
            false,
        )
        .expect("profile-a should exist");
        create_profile(
            &mut config,
            "profile-b",
            profile("https://api.anthropic.com"),
            false,
        )
        .expect("profile-b should exist");

        let err = rename_profile(&mut config, "profile-a", "profile-b");
        assert_err_contains(err, "Profile 'profile-b' already exists");
    }

    #[test]
    fn update_profile_errors_when_profile_missing_before_validating_replacement() {
        let mut config = Config::default();
        let blank_endpoint = Profile::builder()
            .endpoint("   ".to_string())
            .api_key("sk-ant-secret".to_string())
            .fable("claude-fable-5".to_string())
            .opus("claude-opus-4-8".to_string())
            .sonnet("claude-sonnet-4-6".to_string())
            .haiku("claude-haiku-4-5-20251001".to_string())
            .build();

        let err = update_profile(&mut config, "missing", blank_endpoint);
        assert_err_contains(err, "Profile 'missing' does not exist");
    }

    #[test]
    fn delete_profile_errors_when_profile_missing() {
        let mut config = Config::default();
        let err = delete_profile(&mut config, "missing");
        assert_err_contains(err, "Profile 'missing' does not exist");
    }
}
