# cc-profile Design Document

## Overview

**Purpose** — Build a Rust CLI that makes Claude Code endpoint, credential, model, environment, and launch-argument switching explicit and repeatable.

**Current state**

```text
User shell
  ├─ manually exports ANTHROPIC_* env vars
  ├─ remembers model IDs and endpoint values
  └─ runs claude directly
```

**Expected state**

```text
User
  ↓
cc-profile
  ├─ reads ~/.cc-profile
  ├─ selects active profile
  ├─ injects Claude Code env vars
  └─ launches claude with configured args
```

`cc-profile` is a Rust command-line tool for managing Claude Code API configuration profiles. Its goal is to replace manually setting environment variables with a profile-based workflow that can store API endpoints, API keys, model names, custom environment variables, and command-line arguments for launching `claude`.

The primary user experience is interactive. When the user runs:

```bash
cc-profile
```

The CLI opens an interactive menu where the user can view the active profile, manage profiles, edit global launch arguments, edit custom environment variables, and start Claude with the selected configuration.

A secondary non-interactive command interface can be added for scripting, especially for commands such as `cc-profile start`, `cc-profile list`, `cc-profile use <profile>`, and `cc-profile show`.

## Goals

The CLI should allow the user to configure the following without manually exporting environment variables:

- API endpoint (`ANTHROPIC_BASE_URL`)
- API key (`ANTHROPIC_API_KEY`)
- Fable model name (`ANTHROPIC_DEFAULT_FABLE_MODEL`)
- Opus model name (`ANTHROPIC_DEFAULT_OPUS_MODEL`)
- Sonnet model name (`ANTHROPIC_DEFAULT_SONNET_MODEL`)
- Haiku model name (`ANTHROPIC_DEFAULT_HAIKU_MODEL`)
- Optional active session model (`ANTHROPIC_MODEL` or `claude --model <name>`)
- Custom additional environment variables
- Global Claude launch arguments

The CLI should support saving multiple named profiles, selecting an active profile, editing profiles, deleting profiles, and launching Claude using the active profile.

## Non-goals for the first version

The first version does not need to implement a full terminal UI framework. A prompt-based interactive CLI using `dialoguer` is sufficient.

The first version does not need profile-specific custom env vars. Custom env vars are global.

The first version does not need profile-specific Claude arguments. Claude arguments are global.

The first version does not need encrypted config storage, although future versions may store API keys in the OS keychain.

The first version does not support `ANTHROPIC_AUTH_TOKEN`; profiles store API keys only and inject them through `ANTHROPIC_API_KEY`.

## Core Behavior

Running:

```bash
cc-profile
```

enters interactive mode.

Running:

```bash
cc-profile start
```

starts `claude` using the active profile, global custom environment variables, and global Claude arguments.

The active profile supplies the core Claude Code environment variables. The global `envs` section supplies extra custom environment variables. The global `args` section controls command-line arguments passed to `claude`.

## Architecture

**Purpose** — Separate prompt/UI code from profile mutation, validation, persistence, and process launching so each layer can be tested independently.

**Current state**

```text
No implementation exists yet
  ↓
Design only lists proposed files
  ↓
Responsibility boundaries are implicit
```

**Expected state**

```text
main.rs
  ↓
cli.rs / interactive.rs
  ↓
command handlers
  ↓
services
  ├─ ProfileService
  ├─ EnvService
  ├─ ArgsService
  └─ LaunchService
  ↓
ConfigRepository
  ↓
~/.cc-profile
```

The implementation should keep `dialoguer` prompts and `clap` parsing at the outer edge. Domain services should own validation and mutations. `ConfigRepository` should own config path resolution, TOML load/save, file permissions, and migration handling.

This boundary prevents interactive prompts from becoming the only way to exercise profile behavior. Tests should be able to call services directly with in-memory config values and verify deterministic outputs without invoking terminal prompts.

## Interactive Mode

### Main Screen

When the user runs `cc-profile`, the CLI displays the active profile and its configuration.

Example:

```text
cc-profile

Active profile: profile-a

Endpoint: https://api.anthropic.com
API key: sk-ant-••••••••••••abcd
Fable:  fable-model
Opus:   opus-model
Sonnet: sonnet-model
Haiku:  haiku-model

Claude args:
--dangerously-skip-permissions: false

Custom envs:
HTTP_PROXY=http://localhost:7890
HTTPS_PROXY=http://localhost:7890
CUSTOM_FLAG=enabled

? Select an option
> List profiles
  New profile
  Show config
  Args
  Envs
  Start Claude
  Quit
```

The API key should be masked by default. A helper function should show only a small suffix, for example:

```text
sk-ant-••••••••••••abcd
```

If there is no active profile, the main screen should make that clear and guide the user to create or select one.

Example:

```text
cc-profile

No active profile configured.

? Select an option
> New profile
  List profiles
  Show config
  Args
  Envs
  Quit
```

`Start Claude` should only be available when an active profile exists.

## Profile Management

### List Profiles

Selecting `List profiles` shows all saved profiles.

Example:

```text
Profiles

? Select a profile
> profile-a  active
  profile-b
  profile-c
  Back
```

The active profile should be clearly marked.

### Profile Detail View

When the user selects a profile, the CLI shows that profile's configuration.

Example:

```text
Profile: profile-b

Endpoint: https://api.example.com
API key: sk-ant-••••••••••••xyz1
Fable:  custom-fable
Opus:   custom-opus
Sonnet: custom-sonnet
Haiku:  custom-haiku

? Select an option
> Set active
  Edit
  Delete
  Back
```

If the selected profile is already active, the `Set active` option may be hidden or disabled.

### Set Active Profile

When the user selects `Set active`, the CLI updates the top-level `active_profile` field.

Example output:

```text
Profile "profile-b" is now active.
```

After setting the active profile, the CLI should return to the main screen and display the newly active profile.

### New Profile

Selecting `New profile` starts a guided prompt.

Required fields:

```text
Profile name: profile-d
Endpoint: https://api.anthropic.com
API key: sk-ant-...
Fable model: fable-model
Opus model: opus-model
Sonnet model: sonnet-model
Haiku model: haiku-model

? Set as active profile?
> Yes
  No
```

After saving:

```text
Profile "profile-d" saved.
Profile "profile-d" is now active.
```

If the profile name already exists, the CLI should ask before overwriting or require a different name.

Profile names should be validated. A conservative first version can allow letters, numbers, dashes, and underscores.

### Edit Profile

Selecting `Edit` from the profile detail view opens an edit menu.

Example:

```text
Edit profile: profile-b

? What do you want to edit?
> Profile name
  Endpoint
  API key
  Fable model
  Opus model
  Sonnet model
  Haiku model
  Back
```

After editing one field, the CLI should return to the edit menu so the user can change multiple values without navigating back through the full menu tree.

When renaming a profile, the CLI must update `active_profile` if the renamed profile was active.

### Delete Profile

Selecting `Delete` should require confirmation.

Example:

```text
Delete profile "profile-b"? This cannot be undone.
> No
  Yes
```

If the deleted profile is active, the CLI should clear `active_profile` or ask the user to choose a new active profile.

Recommended behavior for the first version:

```text
Profile "profile-b" deleted.
No active profile is currently set.
```

Then return to the main screen.

## Global Args

Claude command-line arguments are stored globally under `[args]`, not per profile.

The first supported argument is:

```toml
[args]
dangerously_skip_permissions = false
```

This maps to the Claude CLI flag:

```bash
--dangerously-skip-permissions
```

When `dangerously_skip_permissions` is `true`, `cc-profile start` should append the flag when launching `claude`.

When it is `false`, the flag should not be included.

### Args Screen

Example:

```text
Args

--dangerously-skip-permissions: false

? Select an option
> Toggle dangerously-skip-permissions
  Back
```

Toggling the value updates the config file immediately.

## Global Custom Environment Variables

Custom additional environment variables are stored globally under `[envs]`, not per profile.

Example:

```toml
[envs]
HTTP_PROXY = "http://localhost:7890"
HTTPS_PROXY = "http://localhost:7890"
CUSTOM_FLAG = "enabled"
```

These values are injected when running:

```bash
cc-profile start
```

They are intended for optional runtime configuration such as proxy settings, feature flags, or provider-specific environment variables.

### Envs Screen

Example:

```text
Custom envs

HTTP_PROXY=http://localhost:7890
HTTPS_PROXY=http://localhost:7890
CUSTOM_FLAG=enabled

? Select an option
> Add env var
  Edit env var
  Delete env var
  Back
```

### Add Env Var

Example:

```text
Env key: HTTP_PROXY
Env value: http://localhost:7890

Saved env var HTTP_PROXY.
```

Environment variable keys should be validated. A conservative first version can require this pattern:

```text
[A-Z_][A-Z0-9_]*
```

### Edit Env Var

Example:

```text
? Select env var
> HTTP_PROXY
  HTTPS_PROXY
  CUSTOM_FLAG

New value: http://localhost:8888

Updated HTTP_PROXY.
```

### Delete Env Var

Example:

```text
? Select env var
> CUSTOM_FLAG

Delete CUSTOM_FLAG?
> No
  Yes

Deleted CUSTOM_FLAG.
```

## Show Config

Selecting `Show config` displays the full current config in a readable format.

Example:

```text
Current config

Config file: ~/.cc-profile
Active profile: profile-a

[args]
dangerously_skip_permissions = false

[envs]
HTTP_PROXY = "http://localhost:7890"
HTTPS_PROXY = "http://localhost:7890"

[profiles.profile-a]
endpoint = "https://api.anthropic.com"
api_key = "sk-ant-••••••••••••abcd"
fable = "fable-model"
opus = "opus-model"
sonnet = "sonnet-model"
haiku = "haiku-model"

? Select an option
> Back
  Reveal API keys
```

API keys should be masked by default. Revealing API keys should require an explicit action.

## Config File

Default config path:

```bash
~/.cc-profile
```

The path is intentionally a fixed file in the user's home directory, not a platform config directory. In Rust, resolve it from the user's home directory and append `.cc-profile`.

### Config Schema

Example full config:

```toml
version = 1
active_profile = "profile-a"

[args]
dangerously_skip_permissions = false

[envs]
HTTP_PROXY = "http://localhost:7890"
HTTPS_PROXY = "http://localhost:7890"
CUSTOM_FLAG = "enabled"

[profiles.profile-a]
endpoint = "https://api.anthropic.com"
api_key = "sk-ant-..."
fable = "fable-model"
opus = "opus-model"
sonnet = "sonnet-model"
haiku = "haiku-model"

[profiles.profile-b]
endpoint = "https://api.example.com"
api_key = "sk-ant-..."
fable = "custom-fable"
opus = "custom-opus"
sonnet = "custom-sonnet"
haiku = "custom-haiku"
```

### Config Versioning

**Purpose** — Make future config migrations explicit and safe.

**Current state**

```text
~/.cc-profile
  └─ no version marker
```

**Expected state**

```text
~/.cc-profile
  ├─ version = 1
  ├─ active_profile
  ├─ args
  ├─ envs
  └─ profiles
```

Missing `version` should be treated as version `1` for backward compatibility with early local files. Unknown newer versions should fail safely with a clear error instead of overwriting the file. Future migrations should load old data into memory, validate the result, and save only after the user performs an action that intentionally writes config.

## Rust Data Model

```rust
use bon::Builder;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize, Builder)]
pub struct Config {
    #[serde(default = "default_config_version")]
    pub version: u32,

    pub active_profile: Option<String>,

    #[serde(default)]
    pub args: Args,

    #[serde(default)]
    pub envs: BTreeMap<String, String>,

    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
}

#[derive(Debug, Serialize, Deserialize, Default, Builder)]
pub struct Args {
    #[serde(default)]
    pub dangerously_skip_permissions: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Builder)]
pub struct Profile {
    pub endpoint: String,
    pub api_key: String,
    pub fable: String,
    pub opus: String,
    pub sonnet: String,
    pub haiku: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: default_config_version(),
            active_profile: None,
            args: Args::default(),
            envs: BTreeMap::new(),
            profiles: BTreeMap::new(),
        }
    }
}

fn default_config_version() -> u32 {
    1
}
```

Use `bon::Builder` for ergonomic and type-safe construction in tests, profile creation flows, and config migration code. For example:

```rust
let profile = Profile::builder()
    .endpoint("https://api.anthropic.com".to_string())
    .api_key("sk-ant-...".to_string())
    .fable("fable-model".to_string())
    .opus("opus-model".to_string())
    .sonnet("sonnet-model".to_string())
    .haiku("haiku-model".to_string())
    .build();
```

For structs with defaultable fields, continue deriving `Default` so loading partial or older config files remains safe. The builder should be used where the code is explicitly constructing new values; serde defaults should still handle missing fields while deserializing.

`BTreeMap` is preferred over `HashMap` so profiles and env vars are displayed in deterministic order.

The first version must keep `bon::Builder`. Builders are part of the design because they make tests and command handlers construct complete `Config`, `Profile`, and `Args` values without long positional constructors or mutable partial state.

## Claude Code Environment Contract

**Purpose** — Launch `claude` with environment variables that Claude Code actually recognizes.

**Current state**

```text
Manual shell setup
  ├─ ANTHROPIC_BASE_URL=...
  ├─ ANTHROPIC_API_KEY=...
  └─ model variables typed by hand
```

**Expected state**

```text
Profile
  ├─ endpoint → ANTHROPIC_BASE_URL
  ├─ api_key  → ANTHROPIC_API_KEY
  ├─ fable    → ANTHROPIC_DEFAULT_FABLE_MODEL
  ├─ opus     → ANTHROPIC_DEFAULT_OPUS_MODEL
  ├─ sonnet   → ANTHROPIC_DEFAULT_SONNET_MODEL
  └─ haiku    → ANTHROPIC_DEFAULT_HAIKU_MODEL
```

Claude Code recognizes `ANTHROPIC_DEFAULT_FABLE_MODEL`, `ANTHROPIC_DEFAULT_OPUS_MODEL`, `ANTHROPIC_DEFAULT_SONNET_MODEL`, and `ANTHROPIC_DEFAULT_HAIKU_MODEL` for family default model IDs. The design must use those `ANTHROPIC_DEFAULT_*_MODEL` names for family model defaults; non-default family variants are not recognized by Claude Code.

The optional session model can be selected by passing `claude --model <alias-or-id>` or by setting `ANTHROPIC_MODEL`. Family defaults should still be populated so aliases resolve correctly through custom gateways.

## Start Behavior

Running:

```bash
cc-profile start
```

loads the config, finds the active profile, injects environment variables, appends configured Claude arguments, and starts `claude`.

Environment variables should be applied in this order:

1. Global custom env vars from `[envs]`
2. Profile-specific Claude env vars derived from the active profile

This order allows profile-specific Claude values to override accidental duplicate custom env vars such as `ANTHROPIC_API_KEY` or `ANTHROPIC_BASE_URL`.

Conceptual command:

```bash
HTTP_PROXY="http://localhost:7890" \
HTTPS_PROXY="http://localhost:7890" \
ANTHROPIC_BASE_URL="https://api.anthropic.com" \
ANTHROPIC_API_KEY="sk-ant-..." \
ANTHROPIC_DEFAULT_FABLE_MODEL="fable-model" \
ANTHROPIC_DEFAULT_OPUS_MODEL="opus-model" \
ANTHROPIC_DEFAULT_SONNET_MODEL="sonnet-model" \
ANTHROPIC_DEFAULT_HAIKU_MODEL="haiku-model" \
claude --dangerously-skip-permissions
```

The `--dangerously-skip-permissions` flag should only be added when this config is true:

```toml
[args]
dangerously_skip_permissions = true
```

### Rust Pseudo-code

```rust
use std::process::Command;

fn start_claude(config: &Config) -> anyhow::Result<()> {
    let active_name = config
        .active_profile
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No active profile is set"))?;

    let profile = config
        .profiles
        .get(active_name)
        .ok_or_else(|| anyhow::anyhow!("Active profile '{}' does not exist", active_name))?;

    let mut cmd = Command::new("claude");

    for (key, value) in &config.envs {
        cmd.env(key, value);
    }

    cmd.env("ANTHROPIC_BASE_URL", &profile.endpoint)
        .env("ANTHROPIC_API_KEY", &profile.api_key)
        .env("ANTHROPIC_DEFAULT_FABLE_MODEL", &profile.fable)
        .env("ANTHROPIC_DEFAULT_OPUS_MODEL", &profile.opus)
        .env("ANTHROPIC_DEFAULT_SONNET_MODEL", &profile.sonnet)
        .env("ANTHROPIC_DEFAULT_HAIKU_MODEL", &profile.haiku);

    if config.args.dangerously_skip_permissions {
        cmd.arg("--dangerously-skip-permissions");
    }

    let status = cmd.status()?;

    if !status.success() {
        anyhow::bail!("claude exited with status: {}", status);
    }

    Ok(())
}
```

## Suggested CLI Commands

The first priority is interactive mode:

```bash
cc-profile
```

Recommended non-interactive commands:

```bash
cc-profile start
cc-profile list
cc-profile use <profile>
cc-profile show
cc-profile new
cc-profile edit <profile>
cc-profile delete <profile>
```

Optional future commands:

```bash
cc-profile env add <KEY> <VALUE>
cc-profile env set <KEY> <VALUE>
cc-profile env delete <KEY>
cc-profile args set dangerously-skip-permissions true
cc-profile export
```

`cc-profile export` could print shell exports without starting Claude:

```bash
export HTTP_PROXY="http://localhost:7890"
export HTTPS_PROXY="http://localhost:7890"
export ANTHROPIC_BASE_URL="https://api.anthropic.com"
export ANTHROPIC_API_KEY="sk-ant-..."
export ANTHROPIC_DEFAULT_FABLE_MODEL="fable-model"
export ANTHROPIC_DEFAULT_OPUS_MODEL="opus-model"
export ANTHROPIC_DEFAULT_SONNET_MODEL="sonnet-model"
export ANTHROPIC_DEFAULT_HAIKU_MODEL="haiku-model"
```

## Recommended Rust Crates

```toml
[dependencies]
anyhow = "1.0.103"
bon = "3.9.2"
clap = { version = "4.6.1", features = ["derive"] }
dialoguer = "0.12.0"
dirs = "6.0.0"
serde = { version = "1.0.228", features = ["derive"] }
toml = "1.1.2"
```

Optional future dependency for storing API keys outside the TOML file:

```toml
keyring = "4.1.2"
```

## Suggested Module Structure

**Purpose** — Group code by architectural boundary so UI, domain rules, persistence, and process execution stay independently testable.

**Current state**

```text
No implementation exists yet
  ↓
A flat module list would mix UI, service logic, config I/O, and process launch code
```

**Expected state**

```text
src/
  main.rs
  cli.rs
  interactive.rs

  config/
    mod.rs
    model.rs
    repository.rs
    validation.rs

  services/
    mod.rs
    profiles.rs
    env_vars.rs
    claude_args.rs
    launch.rs
```

Suggested responsibilities:

```text
main.rs                 Entry point and top-level command dispatch
cli.rs                  clap command definitions for cc-profile commands
interactive.rs          dialoguer-driven menus and prompts only

config/mod.rs           Public config module exports
config/model.rs         Config, Profile, Args, and bon::Builder data types
config/repository.rs    Resolve ~/.cc-profile, load/save TOML, file permissions, version checks
config/validation.rs    Profile names, env var names, endpoint, API key, and model validation

services/mod.rs         Public service module exports
services/profiles.rs    Profile create/edit/delete/rename/set-active mutations
services/env_vars.rs    Global custom environment variable add/edit/delete mutations
services/claude_args.rs Global Claude launch argument mutations
services/launch.rs      Build testable Claude CommandSpec and execute claude
```

The launch service should split command construction from process execution:

```text
Config
  ↓
services::launch::build_command_spec
  ↓
CommandSpec { program, args, envs }
  ↓
services::launch::run_command_spec
  ↓
std::process::Command
```

Tests should assert `CommandSpec` for launch behavior instead of invoking a real `claude` binary.

## Validation Rules

Profile names should be non-empty and should avoid characters that make TOML paths awkward. Recommended first-version pattern:

```text
[a-zA-Z0-9_-]+
```

Environment variable names should be non-empty and shell-friendly. Recommended first-version pattern:

```text
[A-Z_][A-Z0-9_]*
```

Endpoint should be non-empty. Optional validation can require `http://` or `https://`.

API key should be non-empty.

Model names should be non-empty.

## Error Handling

If the config file does not exist, the CLI should start with an empty default config.

If the config file is invalid TOML, the CLI should show a clear error and avoid overwriting the broken file automatically.

If `active_profile` points to a missing profile, the CLI should display a warning and prompt the user to select or create a profile.

If `claude` is not installed or not found on `PATH`, `cc-profile start` should show a clear error:

```text
Could not find `claude` on PATH. Please install Claude Code or ensure the `claude` command is available.
```

If `claude` exits with a non-zero status, `cc-profile` should return a non-zero exit code.

## Security Considerations

**Purpose** — Keep local credential storage explicit, minimally exposed, and easy to migrate to safer storage later.

**Current state**

```text
Manual shell / local files
  ├─ secrets may exist in shell history
  ├─ env vars may leak through process inspection
  └─ no cc-profile-owned storage rules
```

**Expected state**

```text
~/.cc-profile
  ├─ contains credentials in v1
  ├─ created with owner-only permissions on Unix
  ├─ masked in normal display paths
  └─ never written to logs or errors
```

API keys are stored in the config file in the first version. The CLI should warn users that the config file contains secrets and should not be committed to version control.

The config file should be created with `0600` permissions on Unix. If an existing config file has broader permissions, the CLI should warn and offer to fix them before writing more secrets.

Credentials should be masked in all normal interactive views. Revealing credentials should require an explicit user action, and the raw value should never appear in errors, debug output, logs, or test snapshots.

A future version can support storing credentials in the OS keychain using the `keyring` crate. The versioned config schema should make that migration backward-compatible.

## Testing

**Purpose** — Verify behavior at the narrowest useful boundary while keeping full CLI flows covered.

**Current state**

```text
Design acceptance criteria
  └─ no test boundary or command plan
```

**Expected state**

```text
Unit tests
  ├─ validation
  ├─ masking
  ├─ config load/save
  └─ service mutations
Integration tests
  ├─ CLI commands
  ├─ config file behavior
  └─ launch env/args construction
Manual checks
  └─ interactive dialoguer flows
```

Unit tests should cover config path resolution, TOML serialization and deserialization, config version defaults, profile-name validation, environment-variable-name validation, endpoint and API key validation, API key masking, global args mapping, and domain service mutations.

Integration tests should use `assert_cmd`, `assert_fs`, and `predicates` to cover `cc-profile list`, `cc-profile use <profile>`, `cc-profile show`, and `cc-profile start`. Tests for `start` must not invoke a real `claude` binary; they should route through a command-construction seam or a test shim and assert the expected env vars and args.

Manual verification should cover the interactive flows that are hard to assert through stdout alone: create profile, edit profile, rename active profile, delete active profile, toggle args, add/edit/delete env vars, reveal credentials explicitly, and recover from an invalid TOML config without overwriting it.

Final verification for implementation should run:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --workspace
cargo test --doc --workspace
```

If the repository adds `./scripts/ci.sh`, use that as the final verification command.

## First-Version Acceptance Criteria

The first version is complete when the following behaviors work:

- Running `cc-profile` opens interactive mode.
- The main screen displays the active profile and masked config.
- The user can list profiles.
- The active profile is marked in the profile list.
- Selecting a profile shows its config.
- From a selected profile, the user can set active, edit, or delete.
- The user can create a new profile with endpoint, API key, fable, opus, sonnet, and haiku values.
- The user can set a new profile as active.
- The user can view the full config.
- The user can toggle global `args.dangerously_skip_permissions`.
- The user can add, edit, and delete global custom env vars under `[envs]`.
- Running `cc-profile start` launches `claude` with custom envs, active profile envs, and configured args.
- `--dangerously-skip-permissions` is only passed when `args.dangerously_skip_permissions = true`.
