//! Domain services for config mutation and Claude launch behavior.
//!
//! Submodules group env vars, Claude args, profile operations, and [`launch`] (command spec +
//! process start).

pub mod claude_args;
pub mod env_vars;
pub mod launch;
pub mod profiles;
