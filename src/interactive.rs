use crate::config::{Config, ConfigRepository};
use crate::services::launch;
use anyhow::Result;
use dialoguer::Select;

/// Runs the prompt-based interactive menu.
pub fn run() -> Result<()> {
    let repository = ConfigRepository::default()?;
    loop {
        let config = repository.load()?;
        println!("{}", render_main_screen(&config));
        let mut options = vec![
            "List profiles",
            "New profile",
            "Show config",
            "Args",
            "Envs",
        ];
        if active_profile_exists(&config) {
            options.push("Start Claude");
        }
        options.push("Quit");

        let selected = Select::new()
            .with_prompt("Select an option")
            .items(&options)
            .default(0)
            .interact()?;
        match options[selected] {
            "List profiles" => profile_menu(&repository)?,
            "New profile" => new_profile_flow(&repository)?,
            "Show config" => show_config_flow(&repository)?,
            "Args" => args_menu(&repository)?,
            "Envs" => envs_menu(&repository)?,
            "Start Claude" => launch::start_claude(&config)?,
            "Quit" => break,
            _ => unreachable!("menu option should be handled"),
        }
    }
    Ok(())
}

fn render_main_screen(config: &Config) -> String {
    let mut output = String::from("cc-profile\n\n");
    if let Some(active_name) = config.active_profile.as_deref() {
        output.push_str(&format!("Active profile: {active_name}\n\n"));
        if let Some(profile) = config.profiles.get(active_name) {
            output.push_str(&format!("Endpoint: {}\n", profile.endpoint));
            output.push_str(&format!("API key: {}\n", profile.api_key));
            output.push_str(&format!("Fable:  {}\n", profile.fable));
            output.push_str(&format!("Opus:   {}\n", profile.opus));
            output.push_str(&format!("Sonnet: {}\n", profile.sonnet));
            output.push_str(&format!("Haiku:  {}\n\n", profile.haiku));
        } else {
            output.push_str(&format!("Active profile '{active_name}' does not exist.\n"));
            output.push_str("Create or select a profile before starting Claude.\n\n");
        }
    } else {
        output.push_str("No active profile configured.\n\n");
    }
    output.push_str("Claude args:\n");
    output.push_str(&format!(
        "--dangerously-skip-permissions: {}\n\n",
        config.args.dangerously_skip_permissions
    ));
    output.push_str("Custom envs:\n");
    for (key, value) in &config.envs {
        output.push_str(&format!("{key}={value}\n"));
    }
    output
}

fn active_profile_exists(config: &Config) -> bool {
    config
        .active_profile
        .as_ref()
        .is_some_and(|name| config.profiles.contains_key(name))
}

#[allow(dead_code)] // used by profile_menu in Part 4 Task 2
fn profile_options(config: &Config) -> Vec<String> {
    let mut options: Vec<String> = config
        .profiles
        .keys()
        .map(|name| {
            if config.active_profile.as_deref() == Some(name.as_str()) {
                format!("{name}  active")
            } else {
                name.clone()
            }
        })
        .collect();
    options.push("Back".to_string());
    options
}

fn profile_menu(_repository: &ConfigRepository) -> Result<()> {
    Ok(())
}

fn new_profile_flow(_repository: &ConfigRepository) -> Result<()> {
    Ok(())
}

fn show_config_flow(_repository: &ConfigRepository) -> Result<()> {
    Ok(())
}

fn args_menu(_repository: &ConfigRepository) -> Result<()> {
    Ok(())
}

fn envs_menu(_repository: &ConfigRepository) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Profile;
    use std::collections::BTreeMap;

    fn config_with_active_profile() -> Config {
        Config {
            active_profile: Some("profile-a".to_string()),
            envs: BTreeMap::from([(
                "HTTP_PROXY".to_string(),
                "http://localhost:7890".to_string(),
            )]),
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
            ..Default::default()
        }
    }

    #[test]
    fn render_main_screen_shows_active_profile_and_unmasked_api_key() {
        let rendered = render_main_screen(&config_with_active_profile());

        assert!(rendered.contains("Active profile: profile-a"));
        assert!(rendered.contains("API key: sk-ant-secret"));
        assert!(rendered.contains("HTTP_PROXY=http://localhost:7890"));
    }

    #[test]
    fn render_main_screen_warns_when_active_profile_is_missing() {
        let config = Config {
            active_profile: Some("missing-profile".to_string()),
            ..Default::default()
        };

        let rendered = render_main_screen(&config);

        assert!(rendered.contains("Active profile 'missing-profile' does not exist."));
        assert!(rendered.contains("Create or select a profile before starting Claude."));
    }

    #[test]
    fn render_profile_options_marks_active_profile() {
        let options = profile_options(&config_with_active_profile());

        assert_eq!(options, vec!["profile-a  active", "Back"]);
    }
}
