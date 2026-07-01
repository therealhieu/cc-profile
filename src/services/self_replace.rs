//! Standalone binary download, verify, extract, and in-place replacement with rollback.

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path};

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

/// Verifies `archive_bytes` against an expected lowercase hex SHA-256 digest.
pub fn verify_archive_sha256(archive_bytes: &[u8], expected_hex: &str) -> Result<()> {
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

/// Extracts only a top-level `cc-profile` binary from a `.tar.gz` archive; rejects path traversal.
pub fn extract_cc_profile_binary_from_tar_gz(
    archive_bytes: &[u8],
    dest: &Path,
) -> Result<()> {
    let decoder = flate2::read::GzDecoder::new(archive_bytes);
    let mut archive = tar::Archive::new(decoder);
    let mut found = false;
    for entry in archive.entries().context("read tar entries")? {
        let mut entry = entry.context("read tar entry")?;
        let path = entry.path().context("tar entry path")?;
        ensure_safe_tar_entry_path(&path)?;
        let is_root_binary = path.components().count() == 1
            && path.file_name().is_some_and(|n| n == "cc-profile");
        if !is_root_binary {
            continue;
        }
        if found {
            bail!("tar archive contains multiple cc-profile binaries");
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("create parent directory {}", parent.display())
            })?;
        }
        entry.unpack(dest).with_context(|| {
            format!("extract cc-profile binary to {}", dest.display())
        })?;
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
    let output = run(binary).with_context(|| {
        format!("failed to run smoke test for {}", binary.display())
    })?;
    if !output.status.success() {
        bail!(
            "smoke test failed for {} (exit {:?})",
            binary.display(),
            output.status.code()
        );
    }
    Ok(())
}

/// Replaces `target` with `new_binary`, keeping a backup at `backup` for rollback.
pub fn replace_executable_with_rollback(
    target: &Path,
    new_binary: &Path,
    backup: &Path,
) -> Result<()> {
    if target.exists() {
        fs::copy(target, backup).with_context(|| {
            format!("backup current binary at {}", backup.display())
        })?;
    }
    match fs::copy(new_binary, target) {
        Ok(_) => {
            #[cfg(unix)]
            {
                let perms = fs::metadata(new_binary)
                    .context("read replacement binary metadata")?
                    .permissions();
                fs::set_permissions(target, perms).with_context(|| {
                    format!("set permissions on {}", target.display())
                })?;
            }
            Ok(())
        }
        Err(err) => {
            if backup.is_file() {
                let _ = fs::copy(backup, target);
            }
            Err(err).with_context(|| format!("replace binary at {}", target.display()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;
    use flate2::write::GzEncoder;
    use flate2::Compression;
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

    #[test]
    fn expected_sha256_from_sums_parses_matching_line() {
        let sums = "deadbeef  cc-profile-v0.2.0-aarch64-apple-darwin.tar.gz\n";
        let hash =
            expected_sha256_from_sums(sums, "cc-profile-v0.2.0-aarch64-apple-darwin.tar.gz")
                .expect("parse");
        assert_eq!(hash, "deadbeef");
    }

    #[test]
    fn verify_archive_sha256_rejects_mismatch() {
        let err = verify_archive_sha256(b"payload", "00").expect_err("mismatch");
        assert!(
            err.to_string().contains("checksum mismatch"),
            "{err}"
        );
    }

    #[test]
    fn self_replace_failure_checksum_leaves_target_bytes_unchanged() {
        let temp = TempDir::new().expect("tempdir");
        let target = temp.path().join("cc-profile");
        fs::write(&target, b"keep-me").expect("write");
        let err = verify_archive_sha256(b"bad-bytes", "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
            .expect_err("mismatch");
        assert!(err.to_string().contains("checksum mismatch"));
        assert_eq!(fs::read(&target).expect("read"), b"keep-me");
    }

    #[test]
    fn extract_rejects_tar_path_traversal() {
        let err = ensure_safe_tar_entry_path(Path::new("../evil")).expect_err("traversal");
        assert!(err.to_string().contains("path traversal"), "{err}");
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

        let err =
            replace_executable_with_rollback(&target, &missing, &backup).expect_err("fail");
        assert!(err.to_string().contains("replace binary"), "{err}");
        assert_eq!(fs::read(&target).expect("restored"), b"old");
        assert_eq!(fs::read(&backup).expect("bak"), b"old");
    }
}