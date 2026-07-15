use crate::config::{Config, ConfigRepository, Profile};
use crate::services::{claude_args, env_vars, launch, profiles, sync_codex, update};
use anyhow::Result;
use dialoguer::{Confirm, Input, Select};
use std::io::{self, IsTerminal, Write};
use std::path::Path;

/// Runs the prompt-based interactive menu.
pub fn run() -> Result<()> {
    update::run_passive_update_check_before_interactive();
    let repository = ConfigRepository::default()?;
    loop {
        let config = repository.load()?;
        clear_screen()?;
        println!("{}", render_main_screen(&config));
        let options = main_menu_options(&config);

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
            "Sync codex" => sync_codex_flow(&repository)?,
            "Start Claude" => launch::start_claude(&config)?,
            "Start Codex" => launch::start_codex(&config)?,
            "Quit" => break,
            _ => unreachable!("menu option should be handled"),
        }
    }
    Ok(())
}

/// Clears prior interactive output before rendering the next prompt screen.
fn clear_screen() -> Result<()> {
    if !io::stdout().is_terminal() {
        return Ok(());
    }
    print!("\x1B[3J\x1B[2J\x1B[H");
    io::stdout().flush()?;
    Ok(())
}

fn render_main_screen(config: &Config) -> String {
    let mut output = String::from("cc-profile\n");
    output.push_str("────────────────────────────────────────\n");

    if let Some(active_name) = config.active_profile.as_deref() {
        output.push_str(&format!("Active profile  {active_name}\n\n"));
        if let Some(profile) = config.profiles.get(active_name) {
            output.push_str(&format!("Endpoint        {}\n", profile.endpoint));
            output.push_str(&format!("API key         {}\n\n", profile.api_key));
            output.push_str("Models\n");
            output.push_str(&format!("  Fable         {}\n", profile.fable));
            output.push_str(&format!("  Opus          {}\n", profile.opus));
            output.push_str(&format!("  Sonnet        {}\n", profile.sonnet));
            output.push_str(&format!("  Haiku         {}\n\n", profile.haiku));
        } else {
            output.push_str(&format!("Active profile '{active_name}' does not exist.\n"));
            output.push_str("Create or select a profile before starting Claude.\n\n");
        }
    } else {
        output.push_str("No active profile configured.\n\n");
    }

    output.push_str("Claude args\n");
    output.push_str(&format!(
        "  skip permissions  {}\n\n",
        config.args.dangerously_skip_permissions
    ));
    output.push_str("Custom envs\n");
    if config.envs.is_empty() {
        output.push_str("  none\n");
    } else {
        for (key, value) in &config.envs {
            output.push_str(&format!("  {key}={value}\n"));
        }
    }
    output
}

fn active_profile_exists(config: &Config) -> bool {
    config
        .active_profile
        .as_ref()
        .is_some_and(|name| config.profiles.contains_key(name))
}

/// Pure main-menu option list. Start entries appear only when an active profile exists.
fn main_menu_options(config: &Config) -> Vec<&'static str> {
    let mut options = vec![
        "List profiles",
        "New profile",
        "Show config",
        "Args",
        "Envs",
        "Sync codex",
    ];
    if active_profile_exists(config) {
        options.push("Start Claude");
        options.push("Start Codex");
    }
    options.push("Quit");
    options
}

/// Builds (name, label) pairs for the profile menu. Binding the raw profile
/// name to its display label in one value means a profile literally named
/// "foo  active" can't be mangled by stripping a suffix off the label.
fn profile_menu_entries(config: &Config) -> Vec<(String, String)> {
    config
        .profiles
        .keys()
        .map(|name| {
            let label = if config.active_profile.as_deref() == Some(name.as_str()) {
                format!("{name}  active")
            } else {
                name.clone()
            };
            (name.clone(), label)
        })
        .collect()
}

fn profile_menu(repository: &ConfigRepository) -> Result<()> {
    loop {
        let config = repository.load()?;
        let entries = profile_menu_entries(&config);
        let mut labels: Vec<&str> = entries.iter().map(|(_, label)| label.as_str()).collect();
        labels.push("Back");
        let selected = Select::new()
            .with_prompt("Select a profile")
            .items(&labels)
            .default(0)
            .interact()?;
        if selected >= entries.len() {
            return Ok(());
        }
        let profile_name = entries[selected].0.clone();
        profile_detail_menu(repository, &profile_name)?;
    }
}

fn profile_detail_menu(repository: &ConfigRepository, name: &str) -> Result<()> {
    loop {
        let config = repository.load()?;
        let Some(profile) = config.profiles.get(name) else {
            return Ok(());
        };
        clear_screen()?;
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
                repository.update(|config| profiles::set_active_profile(config, name))?;
                println!("Profile \"{name}\" is now active.");
            }
            "Edit" => edit_profile_flow(repository, name)?,
            "Delete" => {
                if Confirm::new()
                    .with_prompt(format!("Delete profile \"{name}\"? This cannot be undone."))
                    .default(false)
                    .interact()?
                {
                    let mut was_active = false;
                    repository.update(|config| {
                        was_active = config.active_profile.as_deref() == Some(name);
                        profiles::delete_profile(config, name)
                    })?;
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
    repository.update(|config| profiles::create_profile(config, &name, profile, set_active))?;
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
        if field == "Profile name" {
            let new_name: String = Input::new()
                .with_prompt("New profile name")
                .interact_text()?;
            repository.update(|config| profiles::rename_profile(config, name, &new_name))?;
            println!("Profile \"{new_name}\" updated.");
            return edit_profile_flow(repository, &new_name);
        }
        let value: String = Input::new()
            .with_prompt(format!("New {field}"))
            .interact_text()?;
        repository.update(|config| {
            let mut profile = config
                .profiles
                .get(name)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Profile '{}' does not exist", name))?;
            apply_profile_field_update(&mut profile, field, &value);
            profiles::update_profile(config, name, profile)
        })?;
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
    clear_screen()?;
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
        clear_screen()?;
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
                let mut enabled = false;
                repository.update(|config| {
                    enabled = claude_args::toggle_dangerously_skip_permissions(config);
                    Ok(())
                })?;
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
        clear_screen()?;
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
                repository.update(|config| env_vars::set_env_var(config, &key, &value))?;
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
    repository.update(|config| env_vars::set_env_var(config, key, &value))?;
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
        repository.update(|config| env_vars::delete_env_var(config, key))?;
        println!("Deleted {key}.");
    }
    Ok(())
}

fn env_options(config: &Config) -> Vec<String> {
    let mut options: Vec<String> = config.envs.keys().cloned().collect();
    options.push("Back".to_string());
    options
}

fn sync_codex_flow(repository: &ConfigRepository) -> Result<()> {
    let config = repository.load()?;
    let path = sync_codex::codex_config_path()?;
    let skipped = sync_codex::sync(&config, &path)?;
    let synced = config.profiles.len() - skipped.len();
    println!("{}", sync_codex_summary(synced, &path, &skipped));
    Ok(())
}

/// Builds the user-facing feedback for a codex sync: one warning line per
/// skipped reserved provider, followed by the synced-count summary. Kept pure
/// (no I/O) so the formatting is unit-testable; `sync_codex_flow` prints it.
fn sync_codex_summary(synced: usize, path: &Path, skipped: &[String]) -> String {
    let mut lines: Vec<String> = skipped
        .iter()
        .map(|name| format!("Skipped profile \"{name}\": reserved Codex provider id"))
        .collect();
    lines.push(format!("Synced {synced} provider(s) to {}", path.display()));
    lines.join("\n")
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

        assert!(rendered.contains("Active profile  profile-a"));
        assert!(rendered.contains("API key         sk-ant-secret"));
        assert!(rendered.contains("  HTTP_PROXY=http://localhost:7890"));
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
    fn profile_menu_entries_binds_name_to_label_without_parsing() {
        let mut config = config_with_active_profile();
        config.profiles.insert(
            "foo  active".to_string(),
            Profile::builder()
                .endpoint("https://api.anthropic.com".to_string())
                .api_key("sk-ant-other".to_string())
                .fable("claude-fable-5".to_string())
                .opus("claude-opus-4-8".to_string())
                .sonnet("claude-sonnet-4-6".to_string())
                .haiku("claude-haiku-4-5-20251001".to_string())
                .build(),
        );

        let entries = profile_menu_entries(&config);

        // A profile literally named "foo  active" must keep its raw name intact
        // (not mangled by stripping a "  active" suffix) and resolve by index,
        // while the actually-active profile gets the " active" label appended.
        assert_eq!(
            entries,
            vec![
                ("foo  active".to_string(), "foo  active".to_string()),
                ("profile-a".to_string(), "profile-a  active".to_string()),
            ]
        );
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
    fn sync_codex_summary_lists_skipped_warnings_then_summary() {
        let skipped = vec!["openai".to_string(), "azure".to_string()];

        let summary = sync_codex_summary(
            3,
            std::path::Path::new("/home/user/.codex/config.toml"),
            &skipped,
        );

        assert_eq!(
            summary,
            "Skipped profile \"openai\": reserved Codex provider id\n\
             Skipped profile \"azure\": reserved Codex provider id\n\
             Synced 3 provider(s) to /home/user/.codex/config.toml"
        );
    }

    #[test]
    fn sync_codex_summary_without_skipped_shows_only_summary() {
        let summary = sync_codex_summary(2, std::path::Path::new("/tmp/config.toml"), &[]);

        assert_eq!(summary, "Synced 2 provider(s) to /tmp/config.toml");
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

    #[test]
    fn main_menu_options_include_start_entries_when_active_profile_exists() {
        let options = main_menu_options(&config_with_active_profile());

        assert_eq!(
            options,
            vec![
                "List profiles",
                "New profile",
                "Show config",
                "Args",
                "Envs",
                "Sync codex",
                "Start Claude",
                "Start Codex",
                "Quit",
            ]
        );
    }

    #[test]
    fn main_menu_options_omit_start_entries_without_active_profile() {
        let options = main_menu_options(&Config::default());

        assert_eq!(
            options,
            vec![
                "List profiles",
                "New profile",
                "Show config",
                "Args",
                "Envs",
                "Sync codex",
                "Quit",
            ]
        );
        assert!(!options.contains(&"Start Claude"));
        assert!(!options.contains(&"Start Codex"));
    }

    #[test]
    fn main_menu_options_omit_start_entries_when_active_profile_is_stale() {
        let config = Config {
            active_profile: Some("missing-profile".to_string()),
            ..Default::default()
        };

        let options = main_menu_options(&config);

        assert_eq!(
            options,
            vec![
                "List profiles",
                "New profile",
                "Show config",
                "Args",
                "Envs",
                "Sync codex",
                "Quit",
            ]
        );
        assert!(!options.contains(&"Start Claude"));
        assert!(!options.contains(&"Start Codex"));
    }
}
