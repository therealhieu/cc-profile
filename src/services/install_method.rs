//! How the running `cc-profile` binary was installed.

use crate::services::receipt::receipt_file_is_standalone;
use std::path::{Path, PathBuf};

/// Detected installation channel for update delegation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallMethod {
    Homebrew,
    Cargo,
    Standalone,
    Unknown,
}

/// Paths used to detect Cargo installs (injectable for deterministic tests).
#[derive(Debug, Clone, Default)]
pub struct InstallPathContext {
    pub cargo_home: Option<PathBuf>,
    pub home_dir: Option<PathBuf>,
}

impl InstallPathContext {
    /// Reads `CARGO_HOME` and the user home directory from the process environment.
    pub fn from_env() -> Self {
        Self {
            cargo_home: std::env::var_os("CARGO_HOME").map(PathBuf::from),
            home_dir: dirs::home_dir(),
        }
    }

    fn cargo_bin_dir(&self) -> Option<PathBuf> {
        let base = self
            .cargo_home
            .clone()
            .or_else(|| self.home_dir.as_ref().map(|h| h.join(".cargo")))?;
        Some(base.join("bin"))
    }
}

/// Classifies an executable path without reading profile config.
pub fn detect_from_exe_path(exe: &Path, receipt_path: Option<&Path>) -> InstallMethod {
    detect_from_exe_path_with_context(exe, receipt_path, &InstallPathContext::from_env())
}

/// Classifies an executable path using injectable home/cargo paths.
pub fn detect_from_exe_path_with_context(
    exe: &Path,
    receipt_path: Option<&Path>,
    ctx: &InstallPathContext,
) -> InstallMethod {
    if is_homebrew_install(exe) {
        return InstallMethod::Homebrew;
    }

    if is_under_cargo_bin(exe, ctx) {
        return InstallMethod::Cargo;
    }

    if let Some(receipt) = receipt_path {
        if receipt.is_file() && receipt_file_is_standalone(receipt) {
            return InstallMethod::Standalone;
        }
    }

    InstallMethod::Unknown
}

fn is_homebrew_install(exe: &Path) -> bool {
    let path_str = exe.to_string_lossy();
    path_str.contains("/Cellar/cc-profile/")
        || path_str.contains("/opt/homebrew/Cellar/cc-profile/")
        || path_str.contains("/usr/local/Cellar/cc-profile/")
        || path_str.contains("/opt/cc-profile/")
        || (path_str.contains("/opt/homebrew/bin/cc-profile") && !path_str.contains("/.cargo/"))
        || path_str.ends_with("/opt/homebrew/opt/cc-profile/bin/cc-profile")
        || path_str.ends_with("/usr/local/opt/cc-profile/bin/cc-profile")
}

fn is_under_cargo_bin(exe: &Path, ctx: &InstallPathContext) -> bool {
    let Some(cargo_bin) = ctx.cargo_bin_dir() else {
        return false;
    };
    exe.starts_with(&cargo_bin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use std::path::Path;

    #[test]
    fn detects_homebrew_cellar_path() {
        let exe = Path::new("/opt/homebrew/Cellar/cc-profile/0.1.0/bin/cc-profile");
        assert_eq!(detect_from_exe_path(exe, None), InstallMethod::Homebrew);
    }

    #[test]
    fn detects_homebrew_opt_bin_path() {
        let exe = Path::new("/opt/homebrew/opt/cc-profile/bin/cc-profile");
        assert_eq!(detect_from_exe_path(exe, None), InstallMethod::Homebrew);
    }

    #[test]
    fn detects_cargo_home_bin_path() {
        let exe = Path::new("/Users/dev/.cargo/bin/cc-profile");
        let ctx = InstallPathContext {
            cargo_home: None,
            home_dir: Some(PathBuf::from("/Users/dev")),
        };
        assert_eq!(
            detect_from_exe_path_with_context(exe, None, &ctx),
            InstallMethod::Cargo
        );
    }

    #[test]
    fn detects_custom_cargo_home_bin_path() {
        let exe = Path::new("/custom/cargo/bin/cc-profile");
        let ctx = InstallPathContext {
            cargo_home: Some(PathBuf::from("/custom/cargo")),
            home_dir: None,
        };
        assert_eq!(
            detect_from_exe_path_with_context(exe, None, &ctx),
            InstallMethod::Cargo
        );
    }

    #[test]
    fn does_not_misclassify_unrelated_cargo_bin_segment() {
        let exe = Path::new("/opt/vendor/cargo/bin/cc-profile");
        let ctx = InstallPathContext {
            cargo_home: None,
            home_dir: Some(PathBuf::from("/opt/vendor")),
        };
        assert_eq!(
            detect_from_exe_path_with_context(exe, None, &ctx),
            InstallMethod::Unknown
        );
    }

    #[test]
    fn detects_standalone_from_receipt() {
        let temp = assert_fs::TempDir::new().expect("tempdir");
        temp.child("install.toml")
            .write_str(
                r#"method = "standalone"
source = "github-releases"
installed_version = "0.1.0"
"#,
            )
            .expect("write receipt");
        let exe = temp.path().join(".local/bin/cc-profile");
        let receipt = temp.path().join("install.toml");
        assert_eq!(
            detect_from_exe_path(&exe, Some(&receipt)),
            InstallMethod::Standalone
        );
    }

    #[test]
    fn unknown_when_no_signals() {
        let exe = Path::new("/tmp/cc-profile");
        assert_eq!(detect_from_exe_path(exe, None), InstallMethod::Unknown);
    }
}
