use crate::config::{Config, ConfigRepository, Profile};
use crate::services::{claude_args, env_vars, launch, profiles};
use anyhow::Result;
use dialoguer::{Confirm, Input, Select};

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

fn profile_menu(repository: &ConfigRepository) -> Result<()> {
    loop {
        let config = repository.load()?;
        let options = profile_options(&config);
        let selected = Select::new()
            .with_prompt("Select a profile")
            .items(&options)
            .default(0)
            .interact()?;
        let selected_option = &options[selected];
        if selected_option == "Back" {
            return Ok(());
        }
        let profile_name = selected_option.trim_end_matches("  active").to_string();
        profile_detail_menu(repository, &profile_name)?;
    }
}

fn profile_detail_menu(repository: &ConfigRepository, name: &str) -> Result<()> {
    loop {
        let config = repository.load()?;
        let Some(profile) = config.profiles.get(name) else {
            return Ok(());
        };
        println!("Profile: {name}\n");
        println!("Endpoint: {}", profile.endpoint);
        println!("API key: {}", profile.api_key);
        println!("Fable:  {}", profile.fable);
        println!("Opus:   {}", profile.opus);
        println!("Sonnet: {}", profile.sonnet);
        println!("Haiku:  {}", profile.haiku);

        let mut options = Vec::new();
        if config.active_profile.as_deref() != Some(name) {
            options.push("Set active");
        }
        options.extend(["Edit", "Delete", "Back"]);
        let selected = Select::new()
            .with_prompt("Select an option")
            .items(&options)
            .default(0)
            .interact()?;
        match options[selected] {
            "Set active" => {
                let mut config = repository.load()?;
                profiles::set_active_profile(&mut config, name)?;
                repository.save(&config)?;
                println!("Profile \"{name}\" is now active.");
            }
            "Edit" => edit_profile_flow(repository, name)?,
            "Delete" => {
                if Confirm::new()
                    .with_prompt(format!("Delete profile \"{name}\"? This cannot be undone."))
                    .default(false)
                    .interact()?
                {
                    let mut config = repository.load()?;
                    let was_active = config.active_profile.as_deref() == Some(name);
                    profiles::delete_profile(&mut config, name)?;
                    repository.save(&config)?;
                    println!("Profile \"{name}\" deleted.");
                    if was_active {
                        println!("No active profile is currently set.");
                    }
                    return Ok(());
                }
            }
            "Back" => return Ok(()),
            _ => unreachable!("profile option should be handled"),
        }
    }
}

fn new_profile_flow(repository: &ConfigRepository) -> Result<()> {
    let name: String = Input::new().with_prompt("Profile name").interact_text()?;
    let endpoint: String = Input::new().with_prompt("Endpoint").interact_text()?;
    let api_key: String = Input::new().with_prompt("API key").interact_text()?;
    let fable: String = Input::new().with_prompt("Fable model").interact_text()?;
    let opus: String = Input::new().with_prompt("Opus model").interact_text()?;
    let sonnet: String = Input::new().with_prompt("Sonnet model").interact_text()?;
    let haiku: String = Input::new().with_prompt("Haiku model").interact_text()?;
    let set_active = Confirm::new()
        .with_prompt("Set as active profile?")
        .default(true)
        .interact()?;
    let profile = Profile::builder()
        .endpoint(endpoint)
        .api_key(api_key)
        .fable(fable)
        .opus(opus)
        .sonnet(sonnet)
        .haiku(haiku)
        .build();
    let mut config = repository.load()?;
    profiles::create_profile(&mut config, &name, profile, set_active)?;
    repository.save(&config)?;
    println!("Profile \"{name}\" saved.");
    if set_active {
        println!("Profile \"{name}\" is now active.");
    }
    Ok(())
}

fn edit_profile_flow(repository: &ConfigRepository, name: &str) -> Result<()> {
    loop {
        let options = [
            "Profile name",
            "Endpoint",
            "API key",
            "Fable model",
            "Opus model",
            "Sonnet model",
            "Haiku model",
            "Back",
        ];
        let selected = Select::new()
            .with_prompt(format!("Edit profile: {name}"))
            .items(options)
            .default(0)
            .interact()?;
        let field = options[selected];
        if field == "Back" {
            return Ok(());
        }
        let mut config = repository.load()?;
        if field == "Profile name" {
            let new_name: String = Input::new()
                .with_prompt("New profile name")
                .interact_text()?;
            profiles::rename_profile(&mut config, name, &new_name)?;
            repository.save(&config)?;
            println!("Profile \"{new_name}\" updated.");
            return edit_profile_flow(repository, &new_name);
        }
        let mut profile = config
            .profiles
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' does not exist", name))?;
        let value: String = Input::new()
            .with_prompt(format!("New {field}"))
            .interact_text()?;
        apply_profile_field_update(&mut profile, field, &value);
        profiles::update_profile(&mut config, name, profile)?;
        repository.save(&config)?;
        println!("Profile \"{name}\" updated.");
    }
}

fn apply_profile_field_update(profile: &mut Profile, field: &str, value: &str) {
    match field {
        "Endpoint" => profile.endpoint = value.to_string(),
        "API key" => profile.api_key = value.to_string(),
        "Fable model" => profile.fable = value.to_string(),
        "Opus model" => profile.opus = value.to_string(),
        "Sonnet model" => profile.sonnet = value.to_string(),
        "Haiku model" => profile.haiku = value.to_string(),
        _ => unreachable!("profile edit field should be known"),
    }
}

fn show_config_flow(repository: &ConfigRepository) -> Result<()> {
    let config = repository.load()?;
    println!("Current config\n");
    println!("Config file: {}", repository.path().display());
    println!(
        "Active profile: {}\n",
        config.active_profile.as_deref().unwrap_or("<none>")
    );
    print!("{}", toml::to_string_pretty(&config)?);
    let _ = Select::new()
        .with_prompt("Select an option")
        .items(["Back"])
        .default(0)
        .interact()?;
    Ok(())
}

fn args_menu(repository: &ConfigRepository) -> Result<()> {
    loop {
        let config = repository.load()?;
        println!("Args\n");
        println!(
            "--dangerously-skip-permissions: {}",
            config.args.dangerously_skip_permissions
        );
        let options = ["Toggle dangerously-skip-permissions", "Back"];
        let selected = Select::new()
            .with_prompt("Select an option")
            .items(options)
            .default(0)
            .interact()?;
        match options[selected] {
            "Toggle dangerously-skip-permissions" => {
                let mut config = repository.load()?;
                let enabled = claude_args::toggle_dangerously_skip_permissions(&mut config);
                repository.save(&config)?;
                println!("--dangerously-skip-permissions: {enabled}");
            }
            "Back" => return Ok(()),
            _ => unreachable!("args option should be handled"),
        }
    }
}

fn envs_menu(repository: &ConfigRepository) -> Result<()> {
    loop {
        let config = repository.load()?;
        println!("Custom envs\n");
        for (key, value) in &config.envs {
            println!("{key}={value}");
        }
        let options = ["Add env var", "Edit env var", "Delete env var", "Back"];
        let selected = Select::new()
            .with_prompt("Select an option")
            .items(options)
            .default(0)
            .interact()?;
        match options[selected] {
            "Add env var" => {
                let key: String = Input::new().with_prompt("Env key").interact_text()?;
                let value: String = Input::new().with_prompt("Env value").interact_text()?;
                let mut config = repository.load()?;
                env_vars::set_env_var(&mut config, &key, &value)?;
                repository.save(&config)?;
                println!("Saved env var {key}.");
            }
            "Edit env var" => edit_env_flow(repository)?,
            "Delete env var" => delete_env_flow(repository)?,
            "Back" => return Ok(()),
            _ => unreachable!("env option should be handled"),
        }
    }
}

fn edit_env_flow(repository: &ConfigRepository) -> Result<()> {
    let config = repository.load()?;
    let options = env_options(&config);
    let selected = Select::new()
        .with_prompt("Select env var")
        .items(&options)
        .default(0)
        .interact()?;
    let key = &options[selected];
    if key == "Back" {
        return Ok(());
    }
    let value: String = Input::new().with_prompt("New value").interact_text()?;
    let mut config = repository.load()?;
    env_vars::set_env_var(&mut config, key, &value)?;
    repository.save(&config)?;
    println!("Updated {key}.");
    Ok(())
}

fn delete_env_flow(repository: &ConfigRepository) -> Result<()> {
    let config = repository.load()?;
    let options = env_options(&config);
    let selected = Select::new()
        .with_prompt("Select env var")
        .items(&options)
        .default(0)
        .interact()?;
    let key = &options[selected];
    if key == "Back" {
        return Ok(());
    }
    if Confirm::new()
        .with_prompt(format!("Delete {key}?"))
        .default(false)
        .interact()?
    {
        let mut config = repository.load()?;
        env_vars::delete_env_var(&mut config, key)?;
        repository.save(&config)?;
        println!("Deleted {key}.");
    }
    Ok(())
}

fn env_options(config: &Config) -> Vec<String> {
    let mut options: Vec<String> = config.envs.keys().cloned().collect();
    options.push("Back".to_string());
    options
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

    #[test]
    fn env_options_include_sorted_env_keys_and_back() {
        let config = Config {
            envs: BTreeMap::from([
                ("HTTPS_PROXY".to_string(), "https://proxy".to_string()),
                ("HTTP_PROXY".to_string(), "http://proxy".to_string()),
            ]),
            ..Default::default()
        };

        // BTreeMap key order: HTTPS_PROXY < HTTP_PROXY lexicographically ('S' before '_').
        assert_eq!(
            env_options(&config),
            vec![
                "HTTPS_PROXY".to_string(),
                "HTTP_PROXY".to_string(),
                "Back".to_string(),
            ]
        );
    }

    #[test]
    fn apply_profile_field_update_changes_requested_field_only() {
        let mut profile = Profile::builder()
            .endpoint("https://api.anthropic.com".to_string())
            .api_key("sk-ant-secret".to_string())
            .fable("claude-fable-5".to_string())
            .opus("claude-opus-4-8".to_string())
            .sonnet("claude-sonnet-4-6".to_string())
            .haiku("claude-haiku-4-5-20251001".to_string())
            .build();

        apply_profile_field_update(&mut profile, "Endpoint", "https://api.example.com");

        assert_eq!(profile.endpoint, "https://api.example.com");
        assert_eq!(profile.api_key, "sk-ant-secret");
    }
}
