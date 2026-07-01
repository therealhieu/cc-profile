//! Profile management for Claude Code endpoints and models.

pub mod cli;
pub mod config;
pub mod interactive;
pub mod services;

pub fn run() -> anyhow::Result<()> {
    cli::run()
}
