# cc-profile

Switch between Claude Code endpoints and models with named profiles.

A **profile** bundles an endpoint, API key, and the four model IDs (Fable, Opus, Sonnet, Haiku). Activate one, run `cc-profile start`, and Claude Code launches against that configuration — no manual environment juggling.

## Table of Contents

- [Install](#install)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Commands](#commands)
- [Updating](#updating)
- [Uninstall](#uninstall)
- [Contributing](#contributing)

## Install

**Homebrew**

```bash
brew install therealhieu/tap/cc-profile
```

**Cargo**

```bash
cargo install cc-profile --locked
```

**Standalone** (GitHub Releases)

```bash
curl -fsSL https://raw.githubusercontent.com/therealhieu/cc-profile/master/install.sh | sh
```

The standalone installer verifies `SHA256SUMS` before placing the binary. Override the location with `CC_PROFILE_INSTALL_DIR`, or pass `--dry-run` to preview without downloading.

## Quick Start

```bash
# 1. Create a profile (mark it active with --active)
cc-profile new \
  --name work \
  --endpoint https://api.anthropic.com \
  --api-key sk-ant-... \
  --fable claude-fable-5 \
  --opus claude-opus-4.8 \
  --sonnet claude-sonnet-4.6 \
  --haiku claude-haiku-4.5 \
  --active

# 2. Launch Claude Code with the active profile
cc-profile start
```

Run `cc-profile` with no arguments for an interactive menu.

## Configuration

Config lives at `~/.cc-profile/config.toml`. The CLI manages it for you, but you can edit it directly:

```toml
version = 1
active_profile = "work"

# Optional: extra flags and env vars applied to every `cc-profile start`
[args]
dangerously_skip_permissions = false

[envs]
HTTPS_PROXY = "https://proxy.example.com"

[profiles.work]
endpoint = "https://api.anthropic.com"
api_key = "sk-ant-..."
fable = "claude-fable-5"
opus = "claude-opus-4.8"
sonnet = "claude-sonnet-4.6"
haiku = "claude-haiku-4.5"

[profiles.staging]
endpoint = "https://staging.example.com"
api_key = "sk-staging-..."
fable = "claude-fable-5"
opus = "claude-opus-4.8"
sonnet = "claude-sonnet-4.6"
haiku = "claude-haiku-4.5"
```

## Commands

| Command | Description |
| --- | --- |
| `cc-profile` | Interactive menu (no subcommand) |
| `cc-profile start` | Launch Claude Code with the active profile |
| `cc-profile list` | List profiles, marking the active one |
| `cc-profile use <name>` | Set the active profile |
| `cc-profile show` | Print the current config and its file path |
| `cc-profile new --name … --endpoint … --api-key … --fable … --opus … --sonnet … --haiku … [--active]` | Create a profile |
| `cc-profile edit <name> [--endpoint …] [--api-key …] [--fable …] [--opus …] [--sonnet …] [--haiku …] [--rename …]` | Update fields on a profile |
| `cc-profile delete <name>` | Delete a profile |
| `cc-profile update` | Update cc-profile itself (see below) |

## Updating

```bash
cc-profile update          # update to the latest release (prompts to confirm)
cc-profile update --yes    # skip the prompt
cc-profile update --check  # check for a newer release without installing
```

What `update` does depends on how you installed:

| Install method | Action |
| --- | --- |
| Homebrew | `brew update` + `brew upgrade therealhieu/tap/cc-profile` |
| Cargo | `cargo install cc-profile --locked --force` |
| Standalone | Downloads the release, verifies `SHA256SUMS`, replaces the binary |

Standalone self-update never skips checksum verification. Set `CC_PROFILE_NO_UPDATE_CHECK=1` to disable the once-per-day update notice.

If an update fails: permission errors mean the binary lives in a non-writable directory (reinstall to `~/.local/bin` or use your package manager); a checksum mismatch means you should retry later rather than force-install.

## Uninstall

**Homebrew**

```bash
brew uninstall cc-profile
```

**Cargo**

```bash
cargo uninstall cc-profile
```

**Standalone**

```bash
rm -f "${CC_PROFILE_INSTALL_DIR:-$HOME/.local/bin}/cc-profile"
rm -f "${CC_PROFILE_RECEIPT_DIR:-$HOME/.cc-profile}/install.toml"
```

## Contributing

Run the same checks as CI locally:

```bash
./scripts/ci.sh              # fmt, clippy, test, package, publish-dry-run
./scripts/ci.sh --help       # per-job details
```

**Releases** — pushing a tag `vX.Y.Z` matching `Cargo.toml` `version` triggers [`release.yml`](.github/workflows/release.yml): it runs CI, builds macOS/Linux archives, uploads them plus `SHA256SUMS` to a GitHub Release, and publishes to crates.io. Requires the `CARGO_REGISTRY_TOKEN` Actions secret.

**Homebrew formula** — the tap formula at [`therealhieu/homebrew-tap`](https://github.com/therealhieu/homebrew-tap) is generated from [`packaging/homebrew/cc-profile.rb.tmpl`](packaging/homebrew/cc-profile.rb.tmpl) on each release and installs prebuilt archives (no source build). CI renders the template into the cloned tap, validates it with `brew style` and `brew audit --online` (using the tap-qualified name so only formula cops apply), and runs a real `brew install` before pushing. To render and inspect locally:

```bash
scripts/render-formula.sh <version> <SHA256SUMS> > cc-profile.rb
```

`brew style`/`brew audit` only report correctly against a formula inside a tap, not a bare file — CI handles that validation, so local inspection is a visual check of the rendered output.

One-time setup (PAT secret, first tap push, livecheck safety net) is documented in [`docs/homebrew-automation.md`](docs/homebrew-automation.md).

**Testing internals** — `tests/install_platform_mapping_test.sh` checks the standalone platform mapping. Integration tests can point `CC_PROFILE_CLAUDE_BIN` at a test shim ([`tests/fixtures/cc-profile-test-claude.rs`](tests/fixtures/cc-profile-test-claude.rs)) instead of the real `claude` binary; unset it afterward.
