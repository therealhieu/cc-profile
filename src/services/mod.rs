//! Domain services for config mutation and Claude launch behavior.
//!
//! Submodules group env vars, Claude args, profile operations, and [`launch`] (command spec +
//! process start).

pub mod claude_args;
pub mod env_vars;
pub mod install_method;
pub mod launch;
pub mod profiles;
pub mod receipt;
pub mod release;
pub mod self_replace;
pub mod sync_codex;
pub mod update;
pub mod update_check_cache;
#[cfg(test)]
pub mod update_test_env_lock;
