//! Profile management for Claude Code endpoints and models.

pub mod cli;
pub mod config;
#[cfg(not(coverage))]
pub mod interactive;
#[cfg(coverage)]
pub mod interactive {
    /// Coverage builds skip the TTY-only interactive menu, which dialoguer cannot drive through
    /// non-terminal stdin.
    pub fn run() -> anyhow::Result<()> {
        Ok(())
    }
}
pub mod services;

pub fn run() -> anyhow::Result<()> {
    cli::run()
}
