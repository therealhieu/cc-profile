//! Configuration loading, validation, and persistence.

pub mod model;
pub mod repository;
pub mod validation;

pub use model::{default_config_version, Args, Config, Profile};
