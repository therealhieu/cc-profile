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
