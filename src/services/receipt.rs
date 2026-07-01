//! Standalone install receipt (`install.toml`) parsing.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct InstallReceiptToml {
    method: String,
}

/// Returns `true` when the receipt file declares `method = "standalone"`.
pub fn receipt_file_is_standalone(path: &Path) -> bool {
    match read_standalone_receipt(path) {
        Ok(true) => true,
        Ok(false) | Err(_) => false,
    }
}

/// Parses the receipt and reports whether the install method is standalone.
pub fn read_standalone_receipt(path: &Path) -> Result<bool> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("read install receipt {}", path.display()))?;
    let receipt: InstallReceiptToml =
        toml::from_str(&contents).context("parse install receipt TOML")?;
    Ok(receipt.method == "standalone")
}

/// Resolves `CC_PROFILE_RECEIPT_DIR/install.toml`, defaulting to `~/.cc-profile/install.toml`.
pub fn default_receipt_path_from_env() -> Option<std::path::PathBuf> {
    let dir = std::env::var_os("CC_PROFILE_RECEIPT_DIR")
        .map(std::path::PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".cc-profile")))?;
    Some(dir.join("install.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;

    #[test]
    fn parses_standalone_method_from_toml() {
        let temp = assert_fs::TempDir::new().expect("tempdir");
        temp.child("install.toml")
            .write_str("method = \"standalone\"\n")
            .expect("write");
        assert!(receipt_file_is_standalone(
            &temp.path().join("install.toml")
        ));
    }

    #[test]
    fn default_receipt_path_honors_cc_profile_receipt_dir() {
        let temp = assert_fs::TempDir::new().expect("tempdir");
        let custom = temp.path().join("receipts");
        // SAFETY: test-only env mutation; single-threaded test harness.
        unsafe {
            std::env::set_var("CC_PROFILE_RECEIPT_DIR", &custom);
        }
        let path = default_receipt_path_from_env().expect("path");
        assert_eq!(path, custom.join("install.toml"));
        unsafe {
            std::env::remove_var("CC_PROFILE_RECEIPT_DIR");
        }
    }

    #[test]
    fn rejects_non_standalone_method() {
        let temp = assert_fs::TempDir::new().expect("tempdir");
        temp.child("install.toml")
            .write_str("method = \"cargo\"\n")
            .expect("write");
        assert!(!receipt_file_is_standalone(
            &temp.path().join("install.toml")
        ));
    }
}
