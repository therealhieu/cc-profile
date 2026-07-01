use bon::Builder;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize, Builder, PartialEq, Eq)]
pub struct Config {
    #[serde(default = "default_config_version")]
    pub version: u32,
    pub active_profile: Option<String>,
    #[serde(default)]
    pub args: Args,
    #[serde(default)]
    pub envs: BTreeMap<String, String>,
    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
}

#[derive(Debug, Serialize, Deserialize, Default, Builder, PartialEq, Eq)]
pub struct Args {
    #[serde(default)]
    pub dangerously_skip_permissions: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Builder, PartialEq, Eq)]
pub struct Profile {
    pub endpoint: String,
    pub api_key: String,
    pub fable: String,
    pub opus: String,
    pub sonnet: String,
    pub haiku: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: default_config_version(),
            active_profile: None,
            args: Args::default(),
            envs: BTreeMap::new(),
            profiles: BTreeMap::new(),
        }
    }
}

pub fn default_config_version() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults_to_version_one_when_version_is_missing() {
        let config: Config = toml::from_str(
            r#"
active_profile = "profile-a"

[profiles.profile-a]
endpoint = "https://api.anthropic.com"
api_key = "sk-ant-secret"
fable = "claude-fable-5"
opus = "claude-opus-4-8"
sonnet = "claude-sonnet-4-6"
haiku = "claude-haiku-4-5-20251001"
"#,
        )
        .expect("config should deserialize");

        assert_eq!(config.version, 1);
        assert_eq!(config.active_profile.as_deref(), Some("profile-a"));
    }

    #[test]
    fn config_serializes_profiles_and_envs_in_deterministic_order() {
        let mut config = Config::default();
        config
            .envs
            .insert("HTTPS_PROXY".into(), "https://proxy".into());
        config
            .envs
            .insert("HTTP_PROXY".into(), "http://proxy".into());
        config.profiles.insert(
            "profile-b".into(),
            Profile::builder()
                .endpoint("https://b.example".to_string())
                .api_key("sk-b".to_string())
                .fable("fable-b".to_string())
                .opus("opus-b".to_string())
                .sonnet("sonnet-b".to_string())
                .haiku("haiku-b".to_string())
                .build(),
        );
        config.profiles.insert(
            "profile-a".into(),
            Profile::builder()
                .endpoint("https://a.example".to_string())
                .api_key("sk-a".to_string())
                .fable("fable-a".to_string())
                .opus("opus-a".to_string())
                .sonnet("sonnet-a".to_string())
                .haiku("haiku-a".to_string())
                .build(),
        );

        let rendered = toml::to_string(&config).expect("config should serialize");

        assert!(rendered.find("HTTPS_PROXY").unwrap() < rendered.find("HTTP_PROXY").unwrap());
        assert!(
            rendered.find("profiles.profile-a").unwrap()
                < rendered.find("profiles.profile-b").unwrap()
        );
        assert!(rendered.contains("version = 1"));
    }
}
