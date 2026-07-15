//! Integration-test shim for `cc-profile start codex`; not used in normal CLI flows.

use std::fs;
use std::path::PathBuf;

fn main() {
    let output = std::env::var("CC_PROFILE_TEST_CODEX_OUTPUT").expect("output path env var");
    let args: Vec<String> = std::env::args().skip(1).collect();
    fs::write(PathBuf::from(output), format!("args={args:?}")).expect("write shim output");
}
