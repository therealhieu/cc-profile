# cc-profile

Profile management for Claude Code endpoints and models.

## Install

### Cargo

```bash
cargo install cc-profile --locked
```

### Standalone (GitHub Releases)

```bash
curl -fsSL https://raw.githubusercontent.com/therealhieu/cc-profile/master/install.sh | sh
```

Or clone the repo and run:

```bash
./install.sh
```

Override install location:

```bash
CC_PROFILE_INSTALL_DIR="$HOME/.local/bin" ./install.sh
```

Dry-run (no downloads or writes):

```bash
CC_PROFILE_INSTALL_DIR=/tmp/cc-profile-install ./install.sh --dry-run
```

The installer verifies `SHA256SUMS` before placing the binary and writes `~/.cc-profile/install.toml` with `method = "standalone"`.

### Homebrew

```bash
brew install therealhieu/tap/cc-profile
```

The canonical formula is maintained in [`therealhieu/homebrew-tap`](https://github.com/therealhieu/homebrew-tap). This repository keeps `Formula/cc-profile.rb` in sync as a source-build reference for tap maintainers.

## Uninstall

### Homebrew

```bash
brew uninstall cc-profile
```

### Cargo

```bash
cargo uninstall cc-profile
```

### Standalone

```bash
rm -f "${CC_PROFILE_INSTALL_DIR:-$HOME/.local/bin}/cc-profile"
rm -f "${CC_PROFILE_RECEIPT_DIR:-$HOME/.cc-profile}/install.toml"
```

Remove `~/.local/bin` from your `PATH` if you added it only for `cc-profile`.

## Update

Check for a newer release without installing:

```bash
cc-profile update --check
```

Install the latest version (interactive confirmation; use `--yes` to skip the prompt):

```bash
cc-profile update
cc-profile update --yes
```

| Install method | What `cc-profile update` does |
| --- | --- |
| Homebrew | Runs `brew update` and `brew upgrade therealhieu/tap/cc-profile` |
| Cargo | Runs `cargo install cc-profile --locked --force` |
| Standalone | Downloads the GitHub release archive, verifies `SHA256SUMS`, then replaces the running binary |

Standalone self-update never skips checksum verification. Homebrew and Cargo updates go through their package managers; `cc-profile` does not overwrite those installs by copying a downloaded binary on top of them.

Interactive mode may print a once-per-day notice when a newer version exists. Disable passive checks with:

```bash
export CC_PROFILE_NO_UPDATE_CHECK=1
```

## Troubleshooting updates

| Problem | What to try |
| --- | --- |
| Permission denied replacing the binary | Reinstall to a user-writable directory (for example `~/.local/bin`), or use `brew upgrade` / `cargo install --force` for package-manager installs |
| Checksum mismatch | Do not force-install; retry when the release assets are fixed, or install manually from the GitHub release after verifying `SHA256SUMS` |
| Offline or GitHub API errors | `cc-profile update --check` fails with a message; your installed binary is left unchanged |
| Unknown install method | Use Homebrew, Cargo, or the standalone installer so `~/.cc-profile/install.toml` records `method = "standalone"` |

## Release automation

Pushing a tag `vX.Y.Z` that matches `Cargo.toml` `version` triggers [`.github/workflows/release.yml`](.github/workflows/release.yml):

- Runs `./scripts/ci.sh`
- Builds `aarch64-apple-darwin`, `x86_64-apple-darwin`, and `x86_64-unknown-linux-gnu` release archives
- Uploads `cc-profile-vX.Y.Z-<target>.tar.gz` and `SHA256SUMS` to a GitHub Release
- Publishes the crate to crates.io

Required GitHub Actions secret:

| Secret | Purpose |
| --- | --- |
| `CARGO_REGISTRY_TOKEN` | crates.io publish on release (create at [crates.io/settings/tokens](https://crates.io/settings/tokens)) |

`GITHUB_TOKEN` is provided automatically for creating the GitHub Release.

## Development and testing

Run the same checks as GitHub Actions locally:

```bash
./scripts/ci.sh
```

Individual jobs: `fmt`, `clippy`, `test`, `package`, `publish-dry-run`. Use `./scripts/ci.sh --help` for details.

Platform mapping for the standalone installer is checked by:

```bash
bash tests/install_platform_mapping_test.sh
```

- **`CC_PROFILE_CLAUDE_BIN`** — Production `cc-profile start` reads this variable and launches that executable instead of `claude` when set. Use only for trusted test or debug binaries (for example the integration-test shim); unset it after debugging so launches go back to the real Claude Code CLI.
- **Test Claude shim** — Source lives at `tests/fixtures/cc-profile-test-claude.rs`. Integration tests compile it on demand; it is not installed with `cargo install`. The shim requires **`CC_PROFILE_TEST_CLAUDE_OUTPUT`** to point to a writable file; without it, the shim exits with an error.

### Homebrew formula checks (macOS)

When validating the formula locally, install from the tap (Homebrew rejects bare `Formula/*.rb` paths):

```bash
brew install --build-from-source therealhieu/tap/cc-profile
brew test cc-profile
brew audit --strict --online therealhieu/tap/cc-profile
```