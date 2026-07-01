//! Binary self-update orchestration (check and apply).

use anyhow::Result;

/// Options for an update run.
#[derive(Debug, Clone, Copy)]
pub struct UpdateOptions {
    pub check_only: bool,
    pub skip_confirm: bool,
}

/// Runs update or check-only flow without loading profile config.
pub fn run_update(options: UpdateOptions) -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    match std::env::var("CC_PROFILE_UPDATE_LOOKUP").as_deref() {
        Ok("stub-current") | Err(_) if options.check_only => {
            println!("cc-profile {current} is up to date.");
            Ok(())
        }
        Ok(other) => anyhow::bail!("unsupported CC_PROFILE_UPDATE_LOOKUP value: {other}"),
        Err(_) => anyhow::bail!("update without --check is not implemented yet"),
    }
}