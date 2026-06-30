//! Command-line interface for `cc-profile`: argument parsing and dispatch.

use crate::interactive;
use anyhow::Result;
use clap::{Parser, Subcommand};

/// Root CLI definition; subcommands are optional (no subcommand runs interactive mode).
#[derive(Debug, Parser)]
#[command(
    name = "cc-profile",
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
    match cli.command {
        None => interactive::run(),
        Some(Command::Start) => command_not_ready("start", "Task 4"),
        Some(Command::List) => command_not_ready("list", "Task 2"),
        Some(Command::Use { .. }) => command_not_ready("use", "Task 2"),
        Some(Command::Show) => command_not_ready("show", "Task 2"),
        Some(Command::New { .. }) => command_not_ready("new", "Task 3"),
        Some(Command::Edit { .. }) => command_not_ready("edit", "Task 3"),
        Some(Command::Delete { .. }) => command_not_ready("delete", "Task 3"),
    }
}

fn command_not_ready(command: &str, task: &str) -> Result<()> {
    anyhow::bail!("Command '{command}' is defined for CLI discovery and completed in {task}");
}
