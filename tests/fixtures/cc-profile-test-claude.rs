//! Integration-test shim for `cc-profile start`; not used in normal CLI flows.

use std::fs;
use std::path::PathBuf;

fn main() {
    let output = std::env::var("CC_PROFILE_TEST_CLAUDE_OUTPUT").expect("output path env var");
    let vars = [
        "HTTP_PROXY",
        "ANTHROPIC_BASE_URL",
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_DEFAULT_FABLE_MODEL",
        "ANTHROPIC_DEFAULT_OPUS_MODEL",
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
    ];
    let mut lines = Vec::new();
    lines.push(format!(
        "args={:?}",
        std::env::args().skip(1).collect::<Vec<_>>()
    ));
    for key in vars {
        lines.push(format!("{key}={}", std::env::var(key).unwrap_or_default()));
    }
    fs::write(PathBuf::from(output), lines.join("\n")).expect("write shim output");
}