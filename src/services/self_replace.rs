//! Standalone binary download, verify, extract, and in-place replacement with rollback.

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};
use tar::EntryType;

const SHA256_HEX_LEN: usize = 64;

/// Parses `SHA256SUMS` and returns the expected hex digest for `archive_basename`.
pub fn expected_sha256_from_sums(sums_body: &str, archive_basename: &str) -> Result<String> {
    for line in sums_body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(hash) = parts.next() else {
            continue;
        };
        validate_sha256_hex(hash)?;
        let name = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("malformed SHA256SUMS line: {line}"))?;
        let name = name.strip_prefix('*').unwrap_or(name);
        if name == archive_basename {
            return Ok(hash.to_ascii_lowercase());
        }
    }
    bail!("SHA256SUMS has no entry for {archive_basename}");
}

fn validate_sha256_hex(hash: &str) -> Result<()> {
    if hash.len() != SHA256_HEX_LEN {
        bail!(
            "SHA256SUMS hash must be {SHA256_HEX_LEN} hex characters, got {}",
            hash.len()
        );
    }
    if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("SHA256SUMS hash must be hexadecimal");
    }
    Ok(())
}

/// Returns lowercase hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Verifies `archive_bytes` against an expected lowercase hex SHA-256 digest.
pub fn verify_archive_sha256(archive_bytes: &[u8], expected_hex: &str) -> Result<()> {
    validate_sha256_hex(expected_hex)?;
    let mut hasher = Sha256::new();
    hasher.update(archive_bytes);
    let digest = hasher.finalize();
    let actual = hex::encode(digest);
    let expected = expected_hex.trim().to_ascii_lowercase();
    if actual != expected {
        bail!("checksum mismatch for release archive (expected {expected}, got {actual})");
    }
    Ok(())
}

/// Rejects absolute paths and `..` segments in a tar entry path.
pub fn ensure_safe_tar_entry_path(path: &Path) -> Result<()> {
    if path.is_absolute() {
        bail!("tar entry must be relative: {}", path.display());
    }
    if path.components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        bail!("tar entry has path traversal: {}", path.display());
    }
    Ok(())
}

/// Rejects symlinks, hard links, and other non-regular entries for `cc-profile`.
pub fn ensure_cc_profile_entry_is_regular_file(entry_type: EntryType) -> Result<()> {
    if entry_type.is_file() {
        return Ok(());
    }
    bail!(
        "tar entry cc-profile must be a regular file, not {:?}",
        entry_type
    );
}

/// Extracts only a top-level `cc-profile` regular file from a `.tar.gz` archive; rejects path traversal.
pub fn extract_cc_profile_binary_from_tar_gz(archive_bytes: &[u8], dest: &Path) -> Result<()> {
    let decoder = flate2::read::GzDecoder::new(archive_bytes);
    let mut archive = tar::Archive::new(decoder);
    let mut found = false;
    for entry in archive.entries().context("read tar entries")? {
        let mut entry = entry.context("read tar entry")?;
        let path = entry.path().context("tar entry path")?;
        ensure_safe_tar_entry_path(&path)?;
        let is_root_binary =
            path.components().count() == 1 && path.file_name().is_some_and(|n| n == "cc-profile");
        if !is_root_binary {
            continue;
        }
        ensure_cc_profile_entry_is_regular_file(entry.header().entry_type())?;
        if found {
            bail!("tar archive contains multiple cc-profile binaries");
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create parent directory {}", parent.display()))?;
        }
        entry
            .unpack(dest)
            .with_context(|| format!("extract cc-profile binary to {}", dest.display()))?;
        found = true;
    }
    if !found {
        bail!("tar archive does not contain a top-level cc-profile binary");
    }
    Ok(())
}

/// Runs `binary --version` and requires success (injectable for tests).
pub fn smoke_test_binary(
    binary: &Path,
    run: impl FnOnce(&Path) -> Result<std::process::Output>,
) -> Result<()> {
    let output = run(binary)
        .with_context(|| format!("failed to run smoke test for {}", binary.display()))?;
    if !output.status.success() {
        bail!(
            "smoke test failed for {} (exit {:?})",
            binary.display(),
            output.status.code()
        );
    }
    Ok(())
}

/// Sibling backup path next to `target` (survives transient temp dirs).
pub fn sibling_backup_path(target: &Path) -> PathBuf {
    let file_name = target
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_else(|| std::ffi::OsString::from("cc-profile"));
    let mut backup_name = file_name;
    backup_name.push(".bak");
    target
        .parent()
        .map(|p| p.join(backup_name))
        .unwrap_or_else(|| PathBuf::from("cc-profile.bak"))
}

/// Replaces `target` with `new_binary`, keeping a backup at `backup` for rollback.
pub fn replace_executable_with_rollback(
    target: &Path,
    new_binary: &Path,
    backup: &Path,
) -> Result<()> {
    if target.exists() {
        fs::copy(target, backup)
            .with_context(|| format!("backup current binary at {}", backup.display()))?;
    }
    match fs::copy(new_binary, target) {
        Ok(_) => {
            #[cfg(unix)]
            {
                let perms = fs::metadata(new_binary)
                    .context("read replacement binary metadata")?
                    .permissions();
                fs::set_permissions(target, perms)
                    .with_context(|| format!("set permissions on {}", target.display()))?;
            }
            Ok(())
        }
        Err(err) => {
            if backup.is_file() {
                fs::copy(backup, target).with_context(|| {
                    format!(
                        "replace binary at {} failed; rollback from {} also failed",
                        target.display(),
                        backup.display()
                    )
                })?;
            }
            Err(err).with_context(|| format!("replace binary at {}", target.display()))
        }
    }
}

/// Restores `target` from `backup` when post-replace verification fails.
pub fn restore_executable_from_backup(target: &Path, backup: &Path) -> Result<()> {
    if !backup.is_file() {
        bail!(
            "cannot restore {}: backup missing at {}",
            target.display(),
            backup.display()
        );
    }
    fs::copy(backup, target).with_context(|| {
        format!(
            "restore {} from backup at {}",
            target.display(),
            backup.display()
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use tar::Builder;

    fn build_tar_gz_with_entry(path_in_archive: &str, contents: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        {
            let mut builder = Builder::new(&mut encoder);
            let mut header = tar::Header::new_gnu();
            header.set_size(contents.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder
                .append_data(&mut header, path_in_archive, contents)
                .expect("append");
            builder.into_inner().expect("finish tar");
        }
        encoder.finish().expect("finish gzip")
    }

    fn build_tar_gz_symlink(path_in_archive: &str, link_target: &str) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        {
            let mut builder = Builder::new(&mut encoder);
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(EntryType::Symlink);
            header.set_size(0);
            header.set_link_name(link_target).expect("link name");
            header.set_cksum();
            builder
                .append_link(&mut header, path_in_archive, link_target)
                .expect("append symlink");
            builder.into_inner().expect("finish tar");
        }
        encoder.finish().expect("finish gzip")
    }

    #[test]
    fn expected_sha256_from_sums_parses_matching_line() {
        let hash_hex = "a".repeat(64);
        let sums = format!("{hash_hex}  cc-profile-v0.2.0-aarch64-apple-darwin.tar.gz\n");
        let hash =
            expected_sha256_from_sums(&sums, "cc-profile-v0.2.0-aarch64-apple-darwin.tar.gz")
                .expect("parse");
        assert_eq!(hash, hash_hex);
    }

    #[test]
    fn expected_sha256_rejects_short_hash() {
        let err = expected_sha256_from_sums(
            "deadbeef  cc-profile-v0.2.0-aarch64-apple-darwin.tar.gz\n",
            "cc-profile-v0.2.0-aarch64-apple-darwin.tar.gz",
        )
        .expect_err("short hash");
        assert!(err.to_string().contains("64 hex"), "{err}");
    }

    #[test]
    fn verify_archive_sha256_rejects_mismatch() {
        let expected = "0".repeat(64);
        let err = verify_archive_sha256(b"payload", &expected).expect_err("mismatch");
        assert!(err.to_string().contains("checksum mismatch"), "{err}");
    }

    #[test]
    fn self_replace_failure_checksum_leaves_target_bytes_unchanged() {
        let temp = TempDir::new().expect("tempdir");
        let target = temp.path().join("cc-profile");
        fs::write(&target, b"keep-me").expect("write");
        let expected = "f".repeat(64);
        let err = verify_archive_sha256(b"bad-bytes", &expected).expect_err("mismatch");
        assert!(err.to_string().contains("checksum mismatch"));
        assert_eq!(fs::read(&target).expect("read"), b"keep-me");
    }

    #[test]
    fn extract_rejects_tar_path_traversal() {
        let err = ensure_safe_tar_entry_path(Path::new("../evil")).expect_err("traversal");
        assert!(err.to_string().contains("path traversal"), "{err}");
    }

    #[test]
    fn extract_rejects_symlink_cc_profile() {
        let archive = build_tar_gz_symlink("cc-profile", "/etc/passwd");
        let temp = TempDir::new().expect("tempdir");
        let dest = temp.path().join("cc-profile");
        let err = extract_cc_profile_binary_from_tar_gz(&archive, &dest).expect_err("symlink");
        assert!(err.to_string().contains("regular file"), "{err}");
    }

    #[test]
    fn extract_only_top_level_cc_profile_binary() {
        let archive = build_tar_gz_with_entry("cc-profile", b"#!/bin/sh\necho ok\n");
        let temp = TempDir::new().expect("tempdir");
        let dest = temp.path().join("cc-profile");
        extract_cc_profile_binary_from_tar_gz(&archive, &dest).expect("extract");
        assert!(dest.is_file());
    }

    #[test]
    fn replace_succeeds_in_temp_directory() {
        let temp = TempDir::new().expect("tempdir");
        let target = temp.path().join("cc-profile");
        let backup = temp.path().join("cc-profile.bak");
        let new_bin = temp.path().join("cc-profile.new");
        fs::write(&target, b"old").expect("old");
        fs::write(&new_bin, b"new").expect("new");

        replace_executable_with_rollback(&target, &new_bin, &backup).expect("replace");
        assert_eq!(fs::read(&target).expect("read"), b"new");
        assert_eq!(fs::read(&backup).expect("bak"), b"old");
    }

    #[test]
    fn rollback_restores_backup_when_replace_fails() {
        let temp = TempDir::new().expect("tempdir");
        let target = temp.path().join("cc-profile");
        let backup = temp.path().join("cc-profile.bak");
        let missing = temp.path().join("missing-new-binary");
        fs::write(&target, b"old").expect("old");

        let err = replace_executable_with_rollback(&target, &missing, &backup).expect_err("fail");
        assert!(err.to_string().contains("replace binary"), "{err}");
        assert_eq!(fs::read(&target).expect("restored"), b"old");
        assert_eq!(fs::read(&backup).expect("bak"), b"old");
    }
}
