//! Command-line interface for `cc-profile`: argument parsing and dispatch.

use crate::config::{ConfigRepository, Profile};
use crate::interactive;
use crate::services::{launch, profiles};
use anyhow::Result;
use clap::{Parser, Subcommand};

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
        profile: String,
    },
    Show,
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
}

/// Parses process arguments and runs the matching handler or interactive mode.
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let repository = ConfigRepository::default()?;
    match cli.command {
        None => interactive::run(),
        Some(Command::List) => list_profiles(&repository),
        Some(Command::Use { profile }) => use_profile(&repository, &profile),
        Some(Command::Show) => show_config(&repository),
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
    }
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
    let mut config = repository.load()?;
    profiles::set_active_profile(&mut config, name)?;
    repository.save(&config)?;
    println!("Profile \"{name}\" is now active.");
    Ok(())
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
    let mut config = repository.load()?;
    let profile = Profile::builder()
        .endpoint(input.endpoint)
        .api_key(input.api_key)
        .fable(input.fable)
        .opus(input.opus)
        .sonnet(input.sonnet)
        .haiku(input.haiku)
        .build();
    profiles::create_profile(&mut config, &input.name, profile, input.active)?;
    repository.save(&config)?;
    println!("Profile \"{}\" saved.", input.name);
    if input.active {
        println!("Profile \"{}\" is now active.", input.name);
    }
    Ok(())
}

fn edit_profile_command(repository: &ConfigRepository, input: EditProfileInput) -> Result<()> {
    let mut config = repository.load()?;
    let original_name = input.profile;
    let mut current_name = original_name.clone();

    if let Some(new_name) = input.rename.as_deref() {
        profiles::rename_profile(&mut config, &original_name, new_name)?;
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

    profiles::update_profile(&mut config, &current_name, profile)?;
    repository.save(&config)?;
    println!("Profile \"{current_name}\" updated.");
    Ok(())
}

fn delete_profile_command(repository: &ConfigRepository, name: &str) -> Result<()> {
    let mut config = repository.load()?;
    let was_active = config.active_profile.as_deref() == Some(name);
    profiles::delete_profile(&mut config, name)?;
    repository.save(&config)?;
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
