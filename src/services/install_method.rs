//! How the running `cc-profile` binary was installed.

use std::path::{Path, PathBuf};

/// Detected installation channel for update delegation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallMethod {
    Homebrew,
    Cargo,
    Standalone,
    Unknown,
}

/// Classifies an executable path without reading profile config.
pub fn detect_from_exe_path(exe: &Path, receipt_path: Option<&Path>) -> InstallMethod {
    let path_str = exe.to_string_lossy();

    if path_str.contains("/Cellar/cc-profile/")
        || path_str.contains("/opt/homebrew/Cellar/cc-profile/")
        || path_str.contains("/usr/local/Cellar/cc-profile/")
        || path_str.contains("/opt/cc-profile/")
        || (path_str.contains("/opt/homebrew/bin/cc-profile")
            && !path_str.contains("/.cargo/"))
        || path_str.ends_with("/opt/homebrew/opt/cc-profile/bin/cc-profile")
        || path_str.ends_with("/usr/local/opt/cc-profile/bin/cc-profile")
    {
        return InstallMethod::Homebrew;
    }

    if path_str.contains("/.cargo/bin/")
        || path_str.contains("/cargo/bin/cc-profile")
        || is_under_cargo_home_bin(exe)
    {
        return InstallMethod::Cargo;
    }

    if let Some(receipt) = receipt_path {
        if receipt.is_file() && receipt_file_is_standalone(receipt) {
            return InstallMethod::Standalone;
        }
    }

    InstallMethod::Unknown
}

fn is_under_cargo_home_bin(exe: &Path) -> bool {
    let cargo_home = std::env::var_os("CARGO_HOME").map(PathBuf::from);
    let cargo_bin = cargo_home
        .or_else(|| dirs::home_dir().map(|h| h.join(".cargo")))
        .map(|h| h.join("bin"));
    let Some(cargo_bin) = cargo_bin else {
        return false;
    };
    exe.starts_with(cargo_bin)
}

fn receipt_file_is_standalone(path: &Path) -> bool {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return false;
    };
    contents.contains("method = \"standalone\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use std::path::Path;

    #[test]
    fn detects_homebrew_cellar_path() {
        let exe = Path::new("/opt/homebrew/Cellar/cc-profile/0.1.0/bin/cc-profile");
        assert_eq!(
            detect_from_exe_path(exe, None),
            InstallMethod::Homebrew
        );
    }

    #[test]
    fn detects_homebrew_opt_bin_path() {
        let exe = Path::new("/opt/homebrew/opt/cc-profile/bin/cc-profile");
        assert_eq!(
            detect_from_exe_path(exe, None),
            InstallMethod::Homebrew
        );
    }

    #[test]
    fn detects_cargo_home_bin_path() {
        let exe = Path::new("/Users/dev/.cargo/bin/cc-profile");
        assert_eq!(detect_from_exe_path(exe, None), InstallMethod::Cargo);
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