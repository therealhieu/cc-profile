use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

/// Compiles the test-only Claude shim once per test binary and returns its path.
pub fn test_claude_shim() -> PathBuf {
    static SHIM: OnceLock<PathBuf> = OnceLock::new();
    SHIM.get_or_init(|| {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let source = manifest_dir.join("tests/fixtures/cc-profile-test-claude.rs");
        let out_dir = std::env::var_os("CARGO_TARGET_TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest_dir.join("target"));
        let shim = out_dir.join("cc-profile-test-claude-fixture");
        let output = Command::new("rustc")
            .arg(&source)
            .arg("-o")
            .arg(&shim)
            .output()
            .unwrap_or_else(|e| {
                panic!(
                    "failed to invoke `rustc` to build test Claude shim.\n\
                     source: {}\n\
                     error: {e}\n\
                     Install Rust (rustc) and ensure it is on PATH to run integration tests.",
                    source.display()
                );
            });
        if !output.status.success() {
            panic!(
                "rustc failed to build test Claude shim (status: {})\n\
                 source: {}\n\
                 stdout:\n{}\n\
                 stderr:\n{}",
                output.status,
                source.display(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
        shim
    })
    .clone()
}

/// Compiles the test-only Codex shim once per test binary and returns its path.
pub fn test_codex_shim() -> PathBuf {
    static SHIM: OnceLock<PathBuf> = OnceLock::new();
    SHIM.get_or_init(|| {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let source = manifest_dir.join("tests/fixtures/cc-profile-test-codex.rs");
        let out_dir = std::env::var_os("CARGO_TARGET_TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest_dir.join("target"));
        let shim = out_dir.join("cc-profile-test-codex-fixture");
        let output = Command::new("rustc")
            .arg(&source)
            .arg("-o")
            .arg(&shim)
            .output()
            .unwrap_or_else(|e| {
                panic!(
                    "failed to invoke `rustc` to build test Codex shim.\n\
                     source: {}\n\
                     error: {e}\n\
                     Install Rust (rustc) and ensure it is on PATH to run integration tests.",
                    source.display()
                );
            });
        if !output.status.success() {
            panic!(
                "rustc failed to build test Codex shim (status: {})\n\
                 source: {}\n\
                 stdout:\n{}\n\
                 stderr:\n{}",
                output.status,
                source.display(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
        shim
    })
    .clone()
}
