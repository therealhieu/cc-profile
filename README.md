# cc-profile

Switch between Claude Code endpoints and models with named profiles.

A **profile** bundles an endpoint, API key, and the four model IDs (Fable, Opus, Sonnet, Haiku). Activate one, run `cc-profile start`, and Claude Code launches against that configuration — no manual environment juggling.

## Table of Contents

- [Install](#install)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Commands](#commands)
- [Sync](#sync)
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

Run `cc-profile` with no arguments to open a menu-driven UI (arrow keys + enter) — no flags to remember.

```
cc-profile
────────────────────────────────────────
Active profile  work

Endpoint        https://api.anthropic.com
API key         sk-ant-...

Models
  Fable         claude-fable-5
  Opus          claude-opus-4.8
  Sonnet        claude-sonnet-4.6
  Haiku         claude-haiku-4.5

Claude args
  skip permissions  false

Custom envs
  none

? Select an option ›
❯ List profiles
  New profile
  Show config
  Args
  Envs
  Start Claude
  Start Codex
  Quit
```

**Creating a profile** — select `New profile` and answer the prompts:

```
? Profile name › staging
? Endpoint › https://staging.example.com
? API key › sk-staging-...
? Fable model › claude-fable-5
? Opus model › claude-opus-4.8
? Sonnet model › claude-sonnet-4.6
? Haiku model › claude-haiku-4.5
? Set as active profile? › yes
Profile "staging" saved.
Profile "staging" is now active.
```

**Managing an existing profile** — `List profiles` shows all profiles (the active one marked), then drills into `Set active`, `Edit`, or `Delete`:

```
? Select a profile ›
❯ staging  active
  work
  Back

? Select an option ›
❯ Edit
  Delete
  Back
```

`Args` and `Envs` work the same way — toggle `--dangerously-skip-permissions`, or add/edit/delete custom environment variables applied to every `cc-profile start`.

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
| `cc-profile start` | Launch Claude Code with the active profile (alias of `start claude`) |
| `cc-profile start claude` | Launch Claude Code with the active profile |
| `cc-profile start codex` | Sync providers into Codex config, then launch Codex with the active profile as `model_provider` and its Opus model |
| `cc-profile list` | List profiles, marking the active one |
| `cc-profile use <name>` | Set the active profile |
| `cc-profile show` | Print the current config and its file path |
| `cc-profile show-command` | Print the exact Claude shell command that `start` / `start claude` would run |
| `cc-profile new --name … --endpoint … --api-key … --fable … --opus … --sonnet … --haiku … [--active]` | Create a profile |
| `cc-profile edit <name> [--endpoint …] [--api-key …] [--fable …] [--opus …] [--sonnet …] [--haiku …] [--rename …]` | Update fields on a profile |
| `cc-profile delete <name>` | Delete a profile |
| `cc-profile sync codex` | Write every profile into `~/.codex/config.toml` as a Codex custom provider (preserves other Codex config) |
| `cc-profile update` | Update cc-profile itself (see below) |

`start` / `start claude` launch Claude Code with the active profile’s env vars and args. `start codex` first auto-syncs all profiles into the Codex config ([Sync](#sync)), then runs `codex -c model_provider="<active>" --model "<opus>"`. The API key stays in the `0o600` Codex config `http_headers` only — never on argv. `show-command` remains Claude-only.

## Sync

```bash
cc-profile sync codex
```

`sync codex` writes every cc-profile profile into your user-level Codex config as a custom provider. It targets `~/.codex/config.toml`, or `$CODEX_HOME/config.toml` when `CODEX_HOME` is set. The file is written with `0600` permissions and its parent directory with `0700`.

Each profile becomes a `[model_providers.<name>]` table with exactly three keys:

| Profile field | Codex provider key |
| --- | --- |
| profile name | provider id + `name` |
| `endpoint` | `base_url` |
| `api_key` | `http_headers = { Authorization = "Bearer <api_key>" }` |

No model is synced — Codex custom providers carry no model, so the four model IDs are left out, along with `wire_api` and any other keys (Codex defaults apply).

The merge is format-preserving: only the `[model_providers.<name>]` tables that match a cc-profile profile are overwritten. Everything else in the Codex config is preserved byte-for-byte — other providers, the top-level `model`, `approval_policy`, `[mcp_servers.*]`, and comments. Providers that don't correspond to a cc-profile profile are never deleted.

The reserved Codex provider ids `openai`, `ollama`, and `lmstudio` cannot be custom providers. A profile with one of those names is skipped with a warning; the remaining profiles still sync and the command exits `0`.

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
