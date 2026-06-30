//! Validation for profile names, environment variables, and profile fields.

use anyhow::{bail, Result};

pub fn validate_profile_name(value: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        bail!("Profile name must contain only letters, numbers, dashes, and underscores");
    }
    Ok(())
}

pub fn validate_env_key(value: &str) -> Result<()> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        bail!("Environment variable name must not be empty");
    };

    if !(first == '_' || first.is_ascii_uppercase()) {
        bail!("Environment variable name must start with A-Z or underscore");
    }

    if !chars.all(|ch| ch == '_' || ch.is_ascii_uppercase() || ch.is_ascii_digit()) {
        bail!("Environment variable name must contain only A-Z, 0-9, and underscores");
    }

    Ok(())
}

pub fn validate_required_value(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{label} must not be empty");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_profile_name_accepts_letters_numbers_dash_and_underscore() {
        for value in ["profile", "profile-a", "profile_a", "Profile1"] {
            assert!(
                validate_profile_name(value).is_ok(),
                "{value} should be valid"
            );
        }
    }

    #[test]
    fn validate_profile_name_rejects_empty_spaces_and_toml_path_punctuation() {
        for value in ["", " ", "profile.a", "profile/a", "profile a"] {
            assert!(
                validate_profile_name(value).is_err(),
                "{value:?} should be invalid"
            );
        }
    }

    #[test]
    fn validate_env_key_accepts_shell_friendly_uppercase_names() {
        for value in ["HTTP_PROXY", "HTTPS_PROXY", "CUSTOM_FLAG", "_PRIVATE"] {
            assert!(validate_env_key(value).is_ok(), "{value} should be valid");
        }
    }

    #[test]
    fn validate_env_key_rejects_lowercase_digits_first_and_punctuation() {
        for value in ["", "http_proxy", "1FLAG", "CUSTOM-FLAG", "CUSTOM FLAG"] {
            assert!(
                validate_env_key(value).is_err(),
                "{value:?} should be invalid"
            );
        }
    }

    #[test]
    fn validate_required_value_rejects_blank_values() {
        assert!(validate_required_value("Endpoint", "https://api.anthropic.com").is_ok());
        assert!(validate_required_value("API key", " ").is_err());
    }
}
