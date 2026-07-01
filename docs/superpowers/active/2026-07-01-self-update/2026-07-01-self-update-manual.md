# Manual Verification — cc-profile Self-Update

## Prerequisites

- Rust toolchain with Rust 1.85+.
- GitHub CLI authenticated if testing release creation.
- Homebrew on macOS for formula checks.
- A clean git tree before publish/release checks.

## 1. Local CLI checks

```bash
cargo build
cargo run -- --version
cargo run -- update --check
```

Expected:

- `--version` prints `cc-profile <Cargo.toml version>`.
- `update --check` does not require `~/.cc-profile/config.toml`.
- No profile API keys, endpoints, or model names are printed or sent.

## 2. Cargo package checks

```bash
cargo package --list
cargo publish --dry-run
```

Expected:

- No missing metadata warnings.
- Package includes `Cargo.toml`, `Cargo.lock`, `README.md`, `LICENSE`, `src/**`, and tests as intended.
- Package excludes planning-only docs and `AGENTS.md`.
- Publish dry-run succeeds.

## 3. Production install binary check

```bash
rm -rf /tmp/cc-profile-install-check
cargo install --path . --locked --force --root /tmp/cc-profile-install-check
find /tmp/cc-profile-install-check/bin -maxdepth 1 -type f -print
/tmp/cc-profile-install-check/bin/cc-profile --version
```

Expected:

- Only `cc-profile` is installed.
- `cc-profile-test-claude` is not installed.

## 4. Homebrew formula check

If Homebrew is available:

```bash
brew install --build-from-source ./Formula/cc-profile.rb
brew test cc-profile
brew audit --strict --online cc-profile
cc-profile --version
cc-profile update --check
```

Expected:

- Formula builds from source.
- Formula test passes.
- `cc-profile update` detects Homebrew and delegates to `brew update` plus `brew upgrade therealhieu/tap/cc-profile`.

## 5. Cargo update delegation check

Install with Cargo into a temp root:

```bash
rm -rf /tmp/cc-profile-cargo-root
cargo install --path . --locked --force --root /tmp/cc-profile-cargo-root
PATH="/tmp/cc-profile-cargo-root/bin:$PATH" cc-profile update --check
```

For full delegation testing, use integration tests with a fake `cargo` on `PATH`.

Expected:

- Cargo install is detected when executable is under Cargo bin path or test fixture path.
- `cc-profile update --yes` invokes structured args equivalent to `cargo install cc-profile --locked --force`.

## 6. Standalone self-update check

Use a temp directory and fixture release server or local test harness:

```bash
mkdir -p /tmp/cc-profile-standalone/bin
cp target/debug/cc-profile /tmp/cc-profile-standalone/bin/cc-profile
/tmp/cc-profile-standalone/bin/cc-profile update --check
```

Then run the integration test that serves fixture release metadata and archives.

Expected:

- Correct target asset is selected.
- Archive checksum is verified against `SHA256SUMS`.
- Only the `cc-profile` binary is extracted.
- Existing binary is backed up before replacement.
- Backup is restored if replacement fails.

## 7. Failure checks

Run tests or manual fixtures for:

- offline latest-version lookup
- malformed release metadata
- checksum mismatch
- unsupported platform
- permission denied while replacing binary
- fake Homebrew/Cargo command failure

Expected:

- Current binary remains usable.
- Error message explains the next action.
- No profile config is modified.

## 8. Passive update notice check

```bash
CC_PROFILE_NO_UPDATE_CHECK=1 cargo run
```

Expected:

- Interactive mode starts without update lookup.

Then remove the env var and use stale cache fixture:

```bash
cargo run
```

Expected:

- If a newer release exists, interactive mode prints a notice only.
- It does not install updates automatically.

## 9. Release workflow check

On a test tag or controlled release:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

Expected GitHub Release assets:

```text
cc-profile-vX.Y.Z-aarch64-apple-darwin.tar.gz
cc-profile-vX.Y.Z-x86_64-apple-darwin.tar.gz
cc-profile-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz
SHA256SUMS
```

Expected:

- Tag version matches `Cargo.toml`.
- GitHub Release is created.
- Crate publishes to crates.io if `CARGO_REGISTRY_TOKEN` is configured.
- Homebrew tap update is opened or documented for manual update.
