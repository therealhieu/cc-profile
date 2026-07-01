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
        let status = Command::new("rustc")
            .arg(&source)
            .arg("-o")
            .arg(&shim)
            .status()
            .expect("invoke rustc for test shim");
        assert!(status.success(), "rustc failed to build test Claude shim");
        shim
    })
    .clone()
}