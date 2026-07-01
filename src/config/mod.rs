//! Configuration loading, validation, and persistence.

pub mod model;
pub mod repository;
pub mod validation;

pub use model::{Args, Config, Profile, default_config_version};
pub use repository::ConfigRepository;
