//! Command-line interface for `cc-profile`: argument parsing and dispatch.

use crate::config::{Config, ConfigRepository, Profile};
use crate::interactive;
use crate::services::{launch, profiles, update};
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::io::IsTerminal;

/// Root CLI definition; subcommands are optional (no subcommand runs interactive mode).
#[derive(Debug, Parser)]
#[command(
    name = "cc-profile",
    version,
    about = "Profile Management for Claude Code Endpoints and Models"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Supported subcommands for profile management.
#[derive(Debug, Subcommand)]
pub enum Command {
    Start,
    List,
    Use {
        profile: Option<String>,
    },
    Show,
    ShowCommand,
    New {
        #[arg(long)]
        name: String,
        #[arg(long)]
        endpoint: String,
        #[arg(long)]
        api_key: String,
        #[arg(long)]
        fable: String,
        #[arg(long)]
        opus: String,
        #[arg(long)]
        sonnet: String,
        #[arg(long)]
        haiku: String,
        #[arg(long)]
        active: bool,
    },
    Edit {
        profile: String,
        #[arg(long)]
        endpoint: Option<String>,
        #[arg(long)]
        api_key: Option<String>,
        #[arg(long)]
        fable: Option<String>,
        #[arg(long)]
        opus: Option<String>,
        #[arg(long)]
        sonnet: Option<String>,
        #[arg(long)]
        haiku: Option<String>,
        #[arg(long)]
        rename: Option<String>,
    },
    Delete {
        profile: String,
    },
    Update {
        #[arg(long)]
        check: bool,
        #[arg(long)]
        yes: bool,
    },
}

/// Parses process arguments and runs the matching handler or interactive mode.
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    if let Some(Command::Update { check, yes }) = cli.command {
        return update_command(check, yes);
    }
    let repository = ConfigRepository::default()?;
    match cli.command {
        None => interactive::run(),
        Some(Command::List) => list_profiles(&repository),
        Some(Command::Use { profile }) => match profile {
            Some(profile) => use_profile(&repository, &profile),
            None => use_profile_interactively(&repository),
        },
        Some(Command::Show) => show_config(&repository),
        Some(Command::ShowCommand) => show_command(&repository),
        Some(Command::Start) => start_command(&repository),
        Some(Command::New {
            name,
            endpoint,
            api_key,
            fable,
            opus,
            sonnet,
            haiku,
            active,
        }) => create_profile_command(
            &repository,
            NewProfileInput {
                name,
                endpoint,
                api_key,
                fable,
                opus,
                sonnet,
                haiku,
                active,
            },
        ),
        Some(Command::Edit {
            profile,
            endpoint,
            api_key,
            fable,
            opus,
            sonnet,
            haiku,
            rename,
        }) => edit_profile_command(
            &repository,
            EditProfileInput {
                profile,
                endpoint,
                api_key,
                fable,
                opus,
                sonnet,
                haiku,
                rename,
            },
        ),
        Some(Command::Delete { profile }) => delete_profile_command(&repository, &profile),
        Some(Command::Update { .. }) => unreachable!("update dispatched above"),
    }
}

fn update_command(check: bool, yes: bool) -> Result<()> {
    update::run_update(update::UpdateOptions {
        check_only: check,
        skip_confirm: yes,
    })
}

fn list_profiles(repository: &ConfigRepository) -> Result<()> {
    let config = repository.load()?;
    for name in config.profiles.keys() {
        if config.active_profile.as_deref() == Some(name.as_str()) {
            println!("{name}  active");
        } else {
            println!("{name}");
        }
    }
    Ok(())
}

fn use_profile(repository: &ConfigRepository, name: &str) -> Result<()> {
    repository.update(|config| profiles::set_active_profile(config, name))?;
    println!("Profile \"{name}\" is now active.");
    Ok(())
}

/// A selectable profile: the raw config key plus its display label. Binding the
/// raw name to its label in one value means a profile literally named
/// "foo  active" can't be mangled by stripping a suffix off the label.
struct ProfileSelectionEntry {
    name: String,
    label: String,
}

/// Builds the selectable profile entries in deterministic (BTreeMap key) order.
/// The active profile's label gets a "  active" suffix; all others use the raw name.
fn profile_selection_entries(config: &Config) -> Vec<ProfileSelectionEntry> {
    config
        .profiles
        .keys()
        .map(|name| {
            let label = if config.active_profile.as_deref() == Some(name.as_str()) {
                format!("{name}  active")
            } else {
                name.clone()
            };
            ProfileSelectionEntry {
                name: name.clone(),
                label,
            }
        })
        .collect()
}

/// Default selection index: the active profile's position when it exists in the
/// entries, otherwise 0. This also covers a stale active profile (set but absent
/// from `profiles`).
fn default_profile_index(config: &Config, entries: &[ProfileSelectionEntry]) -> usize {
    config
        .active_profile
        .as_deref()
        .and_then(|active| entries.iter().position(|entry| entry.name == active))
        .unwrap_or(0)
}

/// Fails before any interactive prompt when there are no profiles to choose from.
fn ensure_selectable(config: &Config) -> Result<()> {
    if config.profiles.is_empty() {
        anyhow::bail!("no profiles configured; create one with `cc-profile new` first");
    }
    Ok(())
}

/// Interactive `cc-profile use` selector for the missing-profile path.
fn use_profile_interactively(repository: &ConfigRepository) -> Result<()> {
    let config = repository.load()?;
    ensure_selectable(&config)?;
    if !std::io::stdin().is_terminal() {
        anyhow::bail!(
            "`cc-profile use` requires an interactive terminal; pass a profile name instead"
        );
    }

    let entries = profile_selection_entries(&config);
    let default_index = default_profile_index(&config, &entries);
    let labels: Vec<&str> = entries.iter().map(|entry| entry.label.as_str()).collect();
    let selection = dialoguer::Select::new()
        .with_prompt("Select a profile")
        .items(&labels)
        .default(default_index)
        .interact_opt()?;
    match selection {
        Some(index) => use_profile(repository, &entries[index].name),
        None => Ok(()),
    }
}

fn show_config(repository: &ConfigRepository) -> Result<()> {
    let config = repository.load()?;
    println!("Current config\n");
    println!("Config file: {}", repository.path().display());
    println!(
        "Active profile: {}\n",
        config.active_profile.as_deref().unwrap_or("<none>")
    );
    print!("{}", toml::to_string_pretty(&config)?);
    Ok(())
}

struct NewProfileInput {
    name: String,
    endpoint: String,
    api_key: String,
    fable: String,
    opus: String,
    sonnet: String,
    haiku: String,
    active: bool,
}

struct EditProfileInput {
    profile: String,
    endpoint: Option<String>,
    api_key: Option<String>,
    fable: Option<String>,
    opus: Option<String>,
    sonnet: Option<String>,
    haiku: Option<String>,
    rename: Option<String>,
}

fn create_profile_command(repository: &ConfigRepository, input: NewProfileInput) -> Result<()> {
    let profile = Profile::builder()
        .endpoint(input.endpoint)
        .api_key(input.api_key)
        .fable(input.fable)
        .opus(input.opus)
        .sonnet(input.sonnet)
        .haiku(input.haiku)
        .build();
    repository
        .update(|config| profiles::create_profile(config, &input.name, profile, input.active))?;
    println!("Profile \"{}\" saved.", input.name);
    if input.active {
        println!("Profile \"{}\" is now active.", input.name);
    }
    Ok(())
}

fn edit_profile_command(repository: &ConfigRepository, input: EditProfileInput) -> Result<()> {
    let original_name = input.profile;
    let mut current_name = original_name.clone();

    repository.update(|config| {
        if let Some(new_name) = input.rename.as_deref() {
            profiles::rename_profile(config, &original_name, new_name)?;
            current_name = new_name.to_string();
        }

        let mut profile = config
            .profiles
            .get(&current_name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' does not exist", current_name))?;

        if let Some(value) = input.endpoint {
            profile.endpoint = value;
        }
        if let Some(value) = input.api_key {
            profile.api_key = value;
        }
        if let Some(value) = input.fable {
            profile.fable = value;
        }
        if let Some(value) = input.opus {
            profile.opus = value;
        }
        if let Some(value) = input.sonnet {
            profile.sonnet = value;
        }
        if let Some(value) = input.haiku {
            profile.haiku = value;
        }

        profiles::update_profile(config, &current_name, profile)
    })?;
    println!("Profile \"{current_name}\" updated.");
    Ok(())
}

fn delete_profile_command(repository: &ConfigRepository, name: &str) -> Result<()> {
    let mut was_active = false;
    repository.update(|config| {
        was_active = config.active_profile.as_deref() == Some(name);
        profiles::delete_profile(config, name)
    })?;
    println!("Profile \"{name}\" deleted.");
    if was_active {
        println!("No active profile is currently set.");
    }
    Ok(())
}

fn start_command(repository: &ConfigRepository) -> Result<()> {
    let config = repository.load()?;
    launch::start_claude(&config)
}

fn show_command(repository: &ConfigRepository) -> Result<()> {
    let config = repository.load()?;
    let spec = launch::build_command_spec(&config)?;
    println!("{}", launch::render_command_line(&spec));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn sample_profile() -> Profile {
        Profile::builder()
            .endpoint("https://api.anthropic.com".to_string())
            .api_key("sk-ant-secret".to_string())
            .fable("claude-fable-5".to_string())
            .opus("claude-opus-4-8".to_string())
            .sonnet("claude-sonnet-4-6".to_string())
            .haiku("claude-haiku-4-5-20251001".to_string())
            .build()
    }

    fn config_with_active_profile() -> Config {
        Config {
            active_profile: Some("profile-a".to_string()),
            profiles: BTreeMap::from([("profile-a".to_string(), sample_profile())]),
            ..Default::default()
        }
    }

    #[test]
    fn profile_selection_entries_include_every_profile_in_btreemap_order() {
        let config = Config {
            active_profile: None,
            profiles: BTreeMap::from([
                ("profile-b".to_string(), sample_profile()),
                ("profile-a".to_string(), sample_profile()),
            ]),
            ..Default::default()
        };

        let names: Vec<String> = profile_selection_entries(&config)
            .into_iter()
            .map(|entry| entry.name)
            .collect();

        assert_eq!(
            names,
            vec!["profile-a".to_string(), "profile-b".to_string()]
        );
    }

    #[test]
    fn profile_selection_entries_bind_name_to_label_without_parsing() {
        let mut config = config_with_active_profile();
        config
            .profiles
            .insert("foo  active".to_string(), sample_profile());

        let entries = profile_selection_entries(&config);

        // A profile literally named "foo  active" must keep its raw name intact
        // (not mangled by stripping a "  active" suffix), while the actually-active
        // profile gets the "  active" label appended.
        assert_eq!(entries[0].name, "foo  active");
        assert_eq!(entries[0].label, "foo  active");
        assert_eq!(entries[1].name, "profile-a");
        assert_eq!(entries[1].label, "profile-a  active");
    }

    #[test]
    fn default_profile_index_points_at_active_profile_when_present() {
        let config = Config {
            active_profile: Some("profile-b".to_string()),
            profiles: BTreeMap::from([
                ("profile-a".to_string(), sample_profile()),
                ("profile-b".to_string(), sample_profile()),
            ]),
            ..Default::default()
        };
        let entries = profile_selection_entries(&config);

        assert_eq!(default_profile_index(&config, &entries), 1);
    }

    #[test]
    fn default_profile_index_is_zero_when_no_active_profile() {
        let config = Config {
            active_profile: None,
            profiles: BTreeMap::from([
                ("profile-a".to_string(), sample_profile()),
                ("profile-b".to_string(), sample_profile()),
            ]),
            ..Default::default()
        };
        let entries = profile_selection_entries(&config);

        assert_eq!(default_profile_index(&config, &entries), 0);
    }

    #[test]
    fn default_profile_index_is_zero_when_active_profile_is_stale() {
        let config = Config {
            active_profile: Some("missing".to_string()),
            profiles: BTreeMap::from([
                ("profile-a".to_string(), sample_profile()),
                ("profile-b".to_string(), sample_profile()),
            ]),
            ..Default::default()
        };
        let entries = profile_selection_entries(&config);

        assert_eq!(default_profile_index(&config, &entries), 0);
    }

    #[test]
    fn ensure_selectable_rejects_empty_config_without_writing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("config.toml");
        let repository = ConfigRepository::new(path.clone());

        let error = repository
            .load()
            .and_then(|config| {
                // Guard is asserted directly against the empty config: the empty
                // check must fire before any dialoguer/TTY interaction.
                ensure_selectable(&config)
            })
            .expect_err("empty config should be rejected");

        assert!(
            error.to_string().contains("no profiles configured"),
            "unexpected error: {error}"
        );
        assert!(
            !path.exists(),
            "no config file should be written for empty selection"
        );
    }
}
