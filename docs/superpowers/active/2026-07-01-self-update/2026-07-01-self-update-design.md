# cc-profile Self-Update Design Document

## Overview

**Purpose** — Make `cc-profile` easy to install and keep current, with one user-facing update command that works across Homebrew, Cargo, and standalone binary installs.

**Current state**

```text
User
  ├─ can build/run from source
  ├─ has no documented install path
  ├─ has no release artifacts
  └─ has no way to update from the CLI
```

**Expected state**

```text
User installs cc-profile
  ├─ Homebrew: brew install therealhieu/tap/cc-profile
  ├─ Cargo:    cargo install cc-profile --locked
  └─ Script:   curl .../install.sh | sh

User updates cc-profile
  ↓
cc-profile update
  ├─ detects install method
  ├─ checks latest GitHub/crates.io version
  ├─ delegates to package manager when appropriate
  └─ self-replaces standalone binaries safely
```

The primary UX is a single command:

```bash
cc-profile update
```

The command should update the binary regardless of how the user installed it. Package-manager installs are updated through their package manager. Standalone installs are updated by downloading, verifying, and atomically replacing the current executable.

## Goals

- Publish `cc-profile` as a polished Rust CLI package.
- Support install through:
  - `cargo install cc-profile --locked`
  - `brew install therealhieu/tap/cc-profile`
  - a standalone install script backed by GitHub Release assets
- Add `cc-profile --version`.
- Add `cc-profile update` with:
  - `--check` to report available updates only
  - `--yes` to skip confirmation prompts
- Support standalone binary self-update using GitHub Releases.
- Respect Homebrew and Cargo ownership instead of overwriting package-managed files directly.
- Add release automation that builds signed-or-checksummed assets.
- Keep update checks deterministic, explicit, and testable.

## Non-goals

- Do not silently replace the binary during normal CLI startup.
- Do not update Claude Code itself.
- Do not migrate or modify `~/.cc-profile/config.toml` as part of binary update checks.
- Do not implement a custom package registry.
- Do not require users to know whether they installed via Homebrew, Cargo, or the install script.

## User Experience

### Version

```bash
cc-profile --version
# cc-profile 0.1.0
```

### Check for updates

```bash
cc-profile update --check
```

Output when current:

```text
cc-profile 0.1.0 is up to date.
```

Output when outdated:

```text
cc-profile 0.2.0 is available. Current version: 0.1.0.
Run `cc-profile update` to install it.
```

### Update now

```bash
cc-profile update
```

Flow:

```text
detect install method
→ fetch latest version
→ compare against current binary version
→ confirm update if interactive
→ execute the matching update backend
→ print final version
```

Examples:

```text
Detected Homebrew install.
Running: brew update && brew upgrade therealhieu/tap/cc-profile
```

```text
Detected Cargo install.
Running: cargo install cc-profile --locked --force
```

```text
Detected standalone install.
Downloading cc-profile-v0.2.0-aarch64-apple-darwin.tar.gz
Verifying SHA256SUMS
Replacing /usr/local/bin/cc-profile
Updated cc-profile 0.1.0 → 0.2.0
```

### Non-interactive update

```bash
cc-profile update --yes
```

`--yes` skips confirmation but still fails safely if verification fails.

## Installation Methods

| Method | User command | Update behavior |
|---|---|---|
| Homebrew | `brew install therealhieu/tap/cc-profile` | Delegate to `brew update && brew upgrade therealhieu/tap/cc-profile` |
| Cargo | `cargo install cc-profile --locked` | Delegate to `cargo install cc-profile --locked --force` |
| Standalone | `install.sh` downloads GitHub Release asset | Download verified asset and replace current executable |
| Unknown | copied binary/manual path | Treat as standalone if parent directory is writable; otherwise print manual instructions |

## Install Method Detection

Detection should use `std::env::current_exe()` and canonicalized paths.

```text
current_exe()
  ↓
canonical path
  ├─ under Homebrew prefix / Cellar / opt → Homebrew
  ├─ under ~/.cargo/bin                 → Cargo
  ├─ install receipt says standalone     → Standalone
  └─ otherwise                           → Unknown/Manual
```

### Homebrew detection

Signals:

- executable path contains `/Cellar/cc-profile/`
- executable path is under `$(brew --prefix)/bin` or `$(brew --prefix)/opt/cc-profile`
- `brew` exists and `brew list --versions cc-profile` succeeds

Use structured `Command` arguments, not shell strings.

### Cargo detection

Signals:

- executable path is under `$CARGO_HOME/bin`, defaulting to `~/.cargo/bin`
- `cargo install --list` includes `cc-profile`

### Standalone detection

Signals:

- install receipt exists at `~/.cc-profile/install.toml`
- executable path is not package-manager-owned
- parent directory is writable

Receipt example:

```toml
method = "standalone"
source = "github-releases"
installed_version = "0.1.0"
installed_at = "2026-07-01T00:00:00Z"
```

## Update Architecture

```text
src/cli.rs
  └─ Command::Update { check, yes }
       ↓
src/services/update.rs
  ├─ UpdateService
  ├─ checks latest version
  ├─ chooses backend
  └─ reports outcome
       ↓
src/services/install_method.rs
  └─ Homebrew | Cargo | Standalone | Unknown
       ↓
src/services/release.rs
  ├─ GitHub Release client
  ├─ asset selection
  └─ checksum parsing
       ↓
src/services/self_replace.rs
  ├─ download archive
  ├─ verify checksum
  ├─ extract binary
  ├─ smoke-test new binary
  └─ replace current executable
```

## CLI Changes

Update `src/cli.rs`:

```rust
#[derive(Debug, Parser)]
#[command(
    name = "cc-profile",
    about = "Profile Management for Claude Code Endpoints and Models",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}
```

Add command:

```rust
Update {
    #[arg(long)]
    check: bool,
    #[arg(long)]
    yes: bool,
},
```

Dispatch:

```rust
Some(Command::Update { check, yes }) => update_command(check, yes),
```

`update_command` should not require loading `ConfigRepository`, because binary updates are independent from profile config.

## Version Source

Current binary version:

```rust
env!("CARGO_PKG_VERSION")
```

Latest version sources:

| Install method | Primary latest source | Fallback |
|---|---|---|
| Homebrew | `brew outdated --json=v2 cc-profile` | GitHub latest release |
| Cargo | crates.io package API | GitHub latest release |
| Standalone | GitHub latest release | none |

Version comparison should use semver parsing.

## Standalone Self-Replacement

Standalone update flow:

```text
GET latest release metadata
→ select asset for current target triple
→ download archive to temp dir
→ download SHA256SUMS
→ verify archive digest
→ extract cc-profile binary
→ chmod executable on Unix
→ run extracted `cc-profile --version`
→ rename current binary to backup
→ rename new binary into place
→ run final `cc-profile --version`
→ delete backup
```

Rollback:

```text
if replacement fails after backup exists:
  move backup back to original path
```

Do not delete the old binary until the new binary is in place and has passed the version check.

## Release Artifacts

Each GitHub Release should include:

```text
cc-profile-vX.Y.Z-aarch64-apple-darwin.tar.gz
cc-profile-vX.Y.Z-x86_64-apple-darwin.tar.gz
cc-profile-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz
SHA256SUMS
```

Each archive should include:

```text
cc-profile
LICENSE
README.md
```

Target triples for first release:

- `aarch64-apple-darwin`
- `x86_64-apple-darwin`
- `x86_64-unknown-linux-gnu`

Windows can be added later with `.zip` archives and platform-specific replacement rules.

## Cargo Publishing Requirements

Update `Cargo.toml` package metadata:

```toml
[package]
name = "cc-profile"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
description = "Profile management for Claude Code endpoints and models"
license = "MIT"
repository = "https://github.com/therealhieu/cc-profile"
homepage = "https://github.com/therealhieu/cc-profile"
readme = "README.md"
keywords = ["claude", "cli", "profile", "configuration"]
categories = ["command-line-utilities"]
exclude = [
  "docs/superpowers/**",
  "AGENTS.md",
]
```

Also:

- Add `LICENSE`.
- Keep `Cargo.lock` committed because this is an application crate.
- Prevent the integration-test shim from being installed as a production binary.

## Test Shim Packaging

`src/bin/cc-profile-test-claude.rs` exists for integration tests. It should not be installed for end users.

Recommended change:

```text
move test shim out of src/bin
→ tests/fixtures/cc-profile-test-claude.rs or tests/shims/cc-profile-test-claude.rs
→ compile it only from integration test setup when needed
```

This avoids publishing or installing a second binary from `cargo install cc-profile`.

## Homebrew Formula

Use a separate tap repo:

```text
therealhieu/homebrew-tap
└── Formula/cc-profile.rb
```

Formula shape:

```ruby
class CcProfile < Formula
  desc "Profile management for Claude Code endpoints and models"
  homepage "https://github.com/therealhieu/cc-profile"
  url "https://github.com/therealhieu/cc-profile/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "<source-tarball-sha256>"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", "--locked", "--path", ".", "--root", prefix
  end

  test do
    system "#{bin}/cc-profile", "--version"
  end

  livecheck do
    url :stable
    strategy :github_latest
  end
end
```

The first Homebrew version can build from source. Bottles can be added after the formula is stable.

## Release Automation

Add GitHub Actions workflows:

```text
.github/workflows/ci.yml
.github/workflows/release.yml
```

### CI workflow

Runs on pull requests and pushes:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
cargo publish --dry-run
```

### Release workflow

Runs on tags matching `v*`:

```text
validate tag matches Cargo.toml version
→ run CI checks
→ build release binaries for target matrix
→ create tar.gz archives
→ generate SHA256SUMS
→ create GitHub Release
→ publish crate to crates.io
→ open/update Homebrew tap PR
```

Publishing to crates.io should require a `CARGO_REGISTRY_TOKEN` secret.

Homebrew tap updates should use a separate token with access only to the tap repo.

## Update Check Cache

For optional passive checks, use a separate cache file:

```text
~/.cc-profile/update-check.toml
```

Example:

```toml
last_checked_at = "2026-07-01T00:00:00Z"
latest_seen = "0.2.0"
```

Environment variable escape hatch:

```bash
CC_PROFILE_NO_UPDATE_CHECK=1
```

Passive checks should only print a notice; they should not install updates.

```text
A new cc-profile version is available: 0.2.0.
Run `cc-profile update` to install it.
```

## Dependencies

Add only the dependencies required for update behavior:

```toml
semver = "1"
ureq = { version = "3", features = ["json"] }
serde_json = "1"
sha2 = "0.10"
tar = "0.4"
flate2 = "1"
```

`tempfile` already exists as a dev-dependency; promote it to a normal dependency if standalone updates use temp directories in production code.

Avoid broad async/runtime dependencies for this feature. Update checks are short-lived blocking operations.

## Security Requirements

- Never send profile config, API keys, model names, or custom env vars during update checks.
- Verify downloaded standalone archives against `SHA256SUMS` before extraction.
- Only extract the expected `cc-profile` binary from release archives.
- Reject archives with path traversal entries.
- Use structured `Command` arguments for `brew` and `cargo`; do not invoke shell strings.
- Fail closed if latest release metadata is malformed.
- Confirm before changing files unless `--yes` is passed.
- Preserve rollback backup until replacement succeeds.

## Error Handling

| Failure | Behavior |
|---|---|
| Offline/network timeout | Print actionable message; leave current binary untouched |
| Already current | Exit success |
| Missing package manager | Print manual command or standalone install instructions |
| Checksum mismatch | Abort; delete downloaded files; leave current binary untouched |
| Unsupported platform | Print manual install instructions |
| Permission denied | Suggest running package-manager update or reinstalling to a user-writable path |
| Replacement failure | Restore backup if possible; print recovery path |

## Documentation Updates

Update `README.md` with:

- Recommended install path: Homebrew
- Cargo install path
- Standalone install script
- `cc-profile update`
- `cc-profile update --check`
- Uninstall instructions for each install method
- Troubleshooting for permission/update failures

Recommended README install section:

```bash
brew tap therealhieu/tap
brew install cc-profile
cc-profile --version
```

Update section:

```bash
cc-profile update
```

## Testing Strategy

### Unit tests

- install method detection from fixture paths
- semver comparison
- GitHub release asset selection by target triple
- checksum parsing and mismatch handling
- update decision logic
- shell command construction for Homebrew/Cargo delegation

### Integration tests

- `cc-profile --version` prints package version
- `cc-profile update --check` reports current/outdated using a mocked HTTP server or fixture client
- `cc-profile update --yes` delegates to fake `brew`/`cargo` binaries on `PATH`
- standalone update replaces a temp fixture binary and rolls back on failure
- update command does not require or mutate profile config

### Release verification

Before publish:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
cargo package --list
cargo publish --dry-run
```

For Homebrew:

```bash
brew install --build-from-source ./Formula/cc-profile.rb
brew test cc-profile
brew audit --strict --online cc-profile
```

## Implementation Sequence

1. Package readiness
   - Add Cargo metadata, license, README install docs, Rust 2024 edition, and `--version`.
   - Remove or relocate the production-visible test shim.

2. Release foundation
   - Add CI workflow.
   - Add release workflow for GitHub assets and checksums.
   - Add crates.io publish step.

3. Update command core
   - Add `cc-profile update --check`.
   - Add install method detection.
   - Add latest-version lookup and semver comparison.

4. Package-manager update backends
   - Add Homebrew delegation.
   - Add Cargo delegation.
   - Test with fake binaries on `PATH`.

5. Standalone self-update backend
   - Add GitHub asset download.
   - Add checksum verification.
   - Add safe replacement and rollback.

6. Optional passive notice
   - Add once/day cache.
   - Print update notice only in interactive mode.
   - Respect `CC_PROFILE_NO_UPDATE_CHECK=1`.

## Acceptance Criteria

- `cc-profile --version` works.
- `cc-profile update --check` works without reading profile config.
- `cc-profile update` updates Homebrew installs through Homebrew.
- `cc-profile update` updates Cargo installs through Cargo.
- `cc-profile update` self-replaces standalone installs after checksum verification.
- Failed downloads, checksum mismatches, and replacement errors leave the existing binary usable.
- Release workflow publishes GitHub assets with `SHA256SUMS`.
- README documents install, update, and uninstall paths.
- `cargo publish --dry-run` passes on a clean tree.
