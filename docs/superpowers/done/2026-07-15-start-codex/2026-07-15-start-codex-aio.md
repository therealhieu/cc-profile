# start-codex — All-in-One Plan

> Concise design + implementation plan in one file. TDD. Subagent-friendly.

## Table of Contents

- [Problem](#problem)
- [Goal](#goal)
- [Non-Goals](#non-goals)
- [Current State](#current-state)
- [Expected State](#expected-state)
- [Testing](#testing)
- [Success Criteria](#success-criteria)
- [Project Standards](#project-standards)
- [Implementation Plan](#implementation-plan)

## Problem

- **Issue 1: `start` only launches Claude Code.** — *Evidence:* `src/cli.rs:25` defines bare `Start` with no target; `start_command` (`src/cli.rs:360-363`) always calls `launch::start_claude`. There is no path that launches Codex. — *Solution:* Task 2.1 nests `Start` under a `StartTarget` subcommand (`claude` | `codex`), defaulting bare `start` to Claude for backward compatibility.
- **Issue 2: Codex needs the active profile's provider + opus model at launch.** — *Evidence:* Codex selects a custom provider via `-c model_provider="<id>"` and a model via `--model <id>` (Codex `CliConfigOverrides`: TOML-parse with raw-string fallback). `sync codex` already writes `[model_providers.<profile>]` with `name`/`base_url`/`http_headers`, but never selects the active provider or model at runtime. — *Solution:* Tasks 1.1–1.2 build a Codex `CommandSpec` that uses the active profile name as `model_provider` and the profile's `opus` value as `--model`, then auto-syncs before `exec`.
- **Issue 3: Launching without a fresh provider block fails silently or with a bad Codex error.** — *Evidence:* If the user never ran `cc-profile sync codex` (or the active profile changed since last sync), Codex errors on an unknown `model_provider`. The user must remember a separate sync step. — *Solution:* Task 1.2 calls `sync_codex::sync` before exec (user choice: auto-sync then launch). Reuses the existing all-profiles merge; no second sync path.
- **Issue 4: Interactive menu has no Codex launch entry.** — *Evidence:* `src/interactive.rs:16-27` shows `"Start Claude"` only when an active profile exists; no Codex option. — *Solution:* Task 2.2 adds `"Start Codex"` gated the same way, calling the new `start_codex` path.
- **Issue 5: Docs still describe `start` as Claude-only.** — *Evidence:* `README.md` Commands table (`cc-profile start` → "Launch Claude Code…") and Quick Start menu omit Codex. — *Solution:* Task 3.1 documents `start [claude|codex]`, the auto-sync behavior, and the opus → `--model` mapping.

## Goal

Deliver `cc-profile start [claude|codex]` that launches Claude Code unchanged, or auto-syncs then launches Codex with the active profile as `model_provider` and its `opus` value as `--model`.

## Non-Goals

- Syncing or selecting Fable/Sonnet/Haiku for Codex (opus only, per user decision).
- Writing `wire_api` or any non-managed Codex keys (inherits existing `sync codex` posture).
- A narrower "sync only active profile" path — reuse full `sync_codex::sync`.
- Changing `sync codex` semantics, reserved-id handling, or permission posture.
- Passing the API key on argv (stays in the 0o600 Codex config via `http_headers`).
- Applying `config.envs` / `config.args` to Codex launch (Claude-only). Codex auth stays in synced `http_headers`; argv is only `-c model_provider=…` and `--model`.
- `show-command codex` (or dual-target show-command). `show-command` remains Claude-only (`start` / `start claude`).
- Printing reserved-skip warnings on `start codex` (unlike `sync codex`); start only fails closed if the *active* profile is reserved.
- Making bare `start` require an explicit target (defaults to Claude).
- Cross-platform non-unix launch differences beyond existing `#[cfg(unix)]` exec path.

## Current State

```
cc-profile start
        │
        ▼
cli::start_command(repository)
        │
        ▼
launch::start_claude(config)
        │
        ├─ build_command_spec → program="claude"
        │                       envs=ANTHROPIC_* + global envs
        │                       args=[--dangerously-skip-permissions?]
        └─ exec_command_spec (unix) / run_command_spec

cc-profile sync codex  ──►  sync_codex::sync(config, path)
                            writes [model_providers.<name>] for every profile
                            (name, base_url, http_headers Bearer)

interactive menu: "Start Claude" only (when active profile exists)
```

## Expected State

```
cc-profile start              ─┐
cc-profile start claude       ─┴─► launch::start_claude   (unchanged)

cc-profile start codex
        │
        ▼
cli::start_codex_command(repository)
        │
        ▼
launch::start_codex(config)
        │
        ├─ 1. resolve_active_profile (Err if missing/stale)
        ├─ 2. reject reserved active name (openai/ollama/lmstudio)
        │       fail BEFORE sync/exec — no config write, no process
        ├─ 3. sync_codex::sync(config, codex_path)
        │       (all profiles; other reserved skipped; 0o600/0o700)
        ├─ 4. build_codex_command_spec
        │       program = CC_PROFILE_CODEX_BIN | "codex"
        │       args    = ["-c", "model_provider=\"<active>\"",
        │                  "--model", "<opus>"]
        │       envs    = {}   (key lives in Codex config, not argv/env)
        └─ 5. exec_command_spec (program-agnostic missing-binary message)

interactive menu: "Start Claude" + "Start Codex" (both gated on active profile)
```

## Testing

- **Framework:** Rust built-in `#[test]` + `assert_cmd` / `assert_fs` / `predicates` for integration
- **TDD cycle:** failing test → run (FAIL) → implement → run (PASS) → commit
- **Coverage target:** every new public/pub(crate) function has unit tests; CLI path has integration coverage via a codex shim (mirrors Claude shim)
- **Test files:**
  - `src/services/launch.rs` (unit tests for codex spec + start_codex preconditions)
  - `src/cli.rs` (clap parse coverage if needed; prefer integration)
  - `src/interactive.rs` (`main_menu_options` pure helper unit tests)
  - `tests/integration/launch.rs` (extend: `start` still Claude; `start codex` argv + provider written)
  - `tests/integration/common.rs` (add `test_codex_shim()`)
  - `tests/fixtures/cc-profile-test-codex.rs` (argv-capturing shim)

## Success Criteria

- [ ] `cc-profile start` and `cc-profile start claude` still launch Claude with existing env/args behavior (no regression)
- [ ] `cc-profile start codex` auto-syncs providers into Codex config, then execs `codex -c model_provider="<active>" --model "<opus>"`
- [ ] Missing / stale active profile returns a clear error before any Codex process is started
- [ ] Reserved active-profile name (e.g. `openai`) returns a clear error **before** sync and before any Codex process; launcher is never called; no config write for that failed start path
- [ ] API key never appears on the Codex argv; only in the 0o600 config `http_headers`
- [ ] Interactive menu offers "Start Codex" (and "Start Claude") when an active profile exists; neither Start entry when none
- [ ] README documents `start [claude|codex]`, auto-sync, opus → model mapping; `show-command` clarified as Claude-only
- [ ] Missing `codex` binary error message is program-agnostic (does not say "install Claude Code")
- [ ] All tests pass
- [ ] `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass
- [ ] No placeholders remain

## Project Standards

Cite, do not restate:

- `AGENTS.md` / user global rules (concise, TDD, show code with path:line)
- Existing service layout: flat `pub mod <name>;` in `src/services/mod.rs`, full-path references, no re-exports
- Launch patterns in `src/services/launch.rs`: `CommandSpec`, `build_command_spec_with_program`, `start_claude_with_launcher`, unix `CommandExt::exec`, `CC_PROFILE_CLAUDE_BIN` override
- Sync patterns in `src/services/sync_codex.rs`: `codex_config_path`, `sync`, reserved ids, 0o600/0o700
- Integration harness: `tests/integration/common.rs` `test_claude_shim()` + `CC_PROFILE_CLAUDE_BIN` / `CC_PROFILE_TEST_CLAUDE_OUTPUT`
- MSRV `rust-version = "1.85"` — no let-chains
- Clap derive nested subcommands (see existing `Sync` / `SyncTarget`)

## Implementation Plan

**Hierarchy:** `Phase → Task → Step`

**Parallelism key:** `[P]` = parallel with siblings, `[S]` = sequential, `[S after Task X.Y]` = blocks on X.Y.

### Phase 1: Codex launch core

**Goal:** Pure/unit-testable Codex command construction and auto-sync-then-exec path in `launch.rs`.

#### Task 1.1: `build_codex_command_spec` [S]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Add a pure builder that resolves the active profile and returns a `CommandSpec` for Codex. Extract a private `resolve_active_profile(config) -> Result<(&str, &Profile)>` shared with the Claude builder path to avoid duplicated bail messages. Program comes from `CC_PROFILE_CODEX_BIN` (or an explicit override helper, mirroring Claude) defaulting to `"codex"`. Args are exactly `["-c", "model_provider=\"<name>\"", "--model", "<opus>"]` — the provider value is double-quoted so Codex's TOML parser treats it as a string even for names like `true` or `123` (quoting is for Codex TOML typing, not shell). Envs are empty (no `config.envs` / `config.args`). Does NOT write files, does NOT exec, does NOT call sync.

**Files:**
- Modify: `src/services/launch.rs`
- Test: unit tests in `src/services/launch.rs` `#[cfg(test)]`

**Code Preview:**

```rust
// crucial: shared resolve; quoted provider + opus; empty envs
fn resolve_active_profile(config: &Config) -> Result<(&str, &Profile)> { /* … */ }

pub(crate) fn build_codex_command_spec_with_program(
    config: &Config,
    program_override: Option<String>,
) -> Result<CommandSpec> {
    let (name, profile) = resolve_active_profile(config)?;
    Ok(CommandSpec {
        program: program_override.unwrap_or_else(|| "codex".into()),
        args: vec![
            "-c".into(),
            format!("model_provider=\"{name}\""),
            "--model".into(),
            profile.opus.clone(),
        ],
        envs: BTreeMap::new(),
    })
}
```

**Steps (run by implementer):**

1. Write failing tests covering:
   - Active profile → args `["-c", "model_provider=\"profile-a\"", "--model", "claude-opus-4-8"]`, program `"codex"`, empty envs
   - `program_override` wins over default
   - No active profile → error containing `"No active profile is set"`
   - Active name missing from `profiles` → error containing `"does not exist"`
   - Profile name needing quotes (e.g. `true`) still rendered as `model_provider="true"`
2. Run tests — expect FAIL (functions missing)
3. Implement minimum code: private `resolve_active_profile`; public `build_codex_command_spec` reading `CC_PROFILE_CODEX_BIN`; `build_codex_command_spec_with_program`; refactor Claude builder to call `resolve_active_profile` only if it stays a tiny mechanical change
4. Run tests — expect PASS
5. Commit: `git commit -m "feat(launch): build Codex CommandSpec from active profile opus"`

**Validation (tester):**
- Full test suite passes
- All behaviors in Scope are covered by tests
- No regressions on existing Claude launch tests
- Lint + typecheck pass (`cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`)

#### Task 1.2: `start_codex` auto-sync + exec [S after Task 1.1]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Add `start_codex(config)` that resolves `codex_config_path()` and delegates to a path-injectable seam. The seam (1) resolves active profile via `resolve_active_profile`, (2) rejects reserved active names **before** any sync or launch (no config write, launcher never called), (3) calls `sync_codex::sync(config, codex_path)`, (4) builds the Codex spec, (5) invokes the injectable launcher. Generalize `exec_command_spec` / `run_command_spec` missing-binary text to be program-agnostic (must not say "install Claude Code" when program is `codex`). Does NOT change Claude start semantics. Does NOT print CLI-facing messages (caller owns UX). Does NOT mutate process env in unit tests.

**Files:**
- Modify: `src/services/launch.rs`
- Test: unit tests in `src/services/launch.rs` `#[cfg(test)]` (tempdir path injection — **never** set `CODEX_HOME` in launch unit tests)

**Code Preview:**

```rust
// crucial: path injection for tests; reserved fails BEFORE sync
pub fn start_codex(config: &Config) -> Result<()> {
    let path = sync_codex::codex_config_path()?;
    start_codex_with_path_and_launcher(config, &path, exec_command_spec)
}

pub(crate) fn start_codex_with_path_and_launcher<F>(
    config: &Config,
    codex_path: &Path,
    launch: F,
) -> Result<()>
where
    F: FnOnce(&CommandSpec) -> Result<()>,
{
    let (name, _) = resolve_active_profile(config)?;
    if sync_codex::is_reserved_provider_id(name) {
        bail!("Cannot start Codex: profile '{name}' is a reserved Codex provider id");
    }
    let _skipped = sync_codex::sync(config, codex_path)?;
    launch(&build_codex_command_spec(config)?)
}
```

**Notes:**
- `is_reserved_provider_id` stays `pub(crate)` in `sync_codex.rs`.
- Do **not** reuse `sync_codex`'s private `ENV_LOCK` from `launch` tests — it is module-private and would not serialize cross-module env races.
- Unit tests pass a temp `codex_path` into `start_codex_with_path_and_launcher`. Integration tests may set `CODEX_HOME` on the **child process** via `Command::env` (subprocess-safe).

**Steps (run by implementer):**

1. Write failing tests covering:
   - Happy path: temp `codex_path`, injectable launcher receives expected `CommandSpec`; provider block written to that path
   - Missing active profile fails before launch; launcher not called; path unchanged/absent
   - Reserved active profile (`openai`) fails **before** sync; launcher not called; codex path not written
   - Sync error (invalid existing TOML at injected path) propagates; launcher not called
   - `exec_command_spec` / missing-program message is program-agnostic for a non-claude program name
2. Run tests — expect FAIL
3. Implement path-injectable seam + reserved-before-sync + program-agnostic exec/run error text
4. Run tests — expect PASS
5. Commit: `git commit -m "feat(launch): start Codex after syncing active profile provider"`

**Validation (tester):**
- Full test suite passes
- Auto-sync side effect verified on injected path after happy-path unit test
- Reserved / missing active-profile paths never call launcher and do not write codex config
- Missing-binary message does not hardcode "Claude Code" for arbitrary program names
- No regressions
- Lint + typecheck pass

**Phase 1 End Review:**
- `spec-reviewer` → Phase 1 goal met; tasks match plan; Success Criteria progress
- `code-quality-reviewer` → code style, standards, no placeholders
- Fix findings: `implementer` + `tester`, max 2 iterations, then move to next phase
- **Gate:** pass to next phase after max 2 fix iterations

### Phase 2: CLI + interactive surface

**Goal:** Wire `start [claude|codex]` into clap and the interactive menu.

#### Task 2.1: CLI `Start` subcommand target [S after Task 1.2]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Change `Command::Start` to carry an optional nested target, defaulting bare `start` to Claude. **Unlike `Sync`/`SyncTarget` (required target), `Start` keeps `target: Option<StartTarget>` so bare `start` stays valid.** Dispatch `Claude` → existing `start_command` / `launch::start_claude`; `Codex` → new handler calling `launch::start_codex`. Preserve exit codes and error messages from the service layer. Add integration coverage with a Codex argv-capturing shim (mirror Claude). Does NOT change other subcommands.

**Files:**
- Modify: `src/cli.rs`
- Create: `tests/fixtures/cc-profile-test-codex.rs` (writes `args=[...]` to path from `CC_PROFILE_TEST_CODEX_OUTPUT`)
- Modify: `tests/integration/common.rs` (add `test_codex_shim()` — `OnceLock` + ad-hoc `rustc`, same as Claude; no `Cargo.toml` fixture registration)
- Modify: `tests/integration/launch.rs` (Claude regression + new codex tests)
- Optionally extend: `tests/integration/cli.rs` help text assertions

**Code Preview:**

```rust
// crucial: Option<StartTarget> keeps bare `start` working (unlike required Sync target)
Start {
    #[command(subcommand)]
    target: Option<StartTarget>,
},

#[derive(Debug, Subcommand)]
pub enum StartTarget {
    Claude,
    Codex,
}
```

**Steps (run by implementer):**

1. Write failing tests covering:
   - Integration: existing `start` Claude shim test still passes
   - Integration: `start claude` same as bare `start` (Claude envs)
   - Integration: `start codex` with `HOME` + `CODEX_HOME` + `CC_PROFILE_CODEX_BIN` + `CC_PROFILE_TEST_CODEX_OUTPUT` shim:
     - exit 0
     - shim output contains `-c` and `model_provider="profile-a"` and `--model` + opus value
     - `$CODEX_HOME/config.toml` contains `[model_providers.profile-a]` and `Bearer …`
     - shim argv does **not** contain the api key
   - Integration: `start codex` with no active profile → failure, stderr contains `No active profile`
   - Integration: active profile `openai` → `start codex` fails, stderr contains reserved message, shim output absent, codex config not written (fail-before-sync)
   - Help lists `claude` / `codex` under `start` (optional but preferred)
2. Run tests — expect FAIL
3. Implement clap enum + dispatch + `start_codex_command`; add codex shim fixture + `test_codex_shim()`
4. Run tests — expect PASS
5. Commit: `git commit -m "feat(cli): add start claude|codex subcommand targets"`

**Validation (tester):**
- Full suite passes including new integration tests
- Bare `start` regression green
- Codex path auto-sync + argv contract verified
- Reserved-active integration case green (no write, no shim)
- Lint + typecheck pass

#### Task 2.2: Interactive "Start Codex" menu option [P with Task 2.1 after Task 1.2]

**Note on parallelism:** Task 2.2 only needs Task 1.2's `launch::start_codex`. It can run in parallel with Task 2.1 if implementers touch different files carefully (`interactive.rs` vs `cli.rs`/tests). If merge conflict risk is high, run sequentially after 2.1. Prefer **parallel** when both start from a clean tree post-1.2.

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Extract pure `main_menu_options(config) -> Vec<&'static str>` and unit-test it. When an active profile exists, the list includes both `"Start Claude"` and `"Start Codex"` (after existing options, before `"Quit"`). When no / stale active profile, neither Start entry appears. Selecting `"Start Codex"` calls `launch::start_codex(&config)`. Keep `"Start Claude"` behavior unchanged. No compile-only escape hatch.

**Files:**
- Modify: `src/interactive.rs`
- Test: unit tests in `src/interactive.rs` for `main_menu_options`

**Code Preview:**

```rust
// crucial: pure helper gates both Start entries
fn main_menu_options(config: &Config) -> Vec<&'static str> {
    let mut options = vec![/* List… Sync codex */];
    if active_profile_exists(config) {
        options.push("Start Claude");
        options.push("Start Codex");
    }
    options.push("Quit");
    options
}
```

**Steps (run by implementer):**

1. Write failing tests covering:
   - With active profile: options include `Start Claude`, `Start Codex`, and `Quit`
   - Without active profile / stale active: neither Start entry; still `Quit`
   - Other fixed entries (e.g. `Sync codex`) still present
2. Run tests — expect FAIL
3. Implement `main_menu_options`, wire `run()` to use it, add `"Start Codex"` match arm → `launch::start_codex`
4. Run tests — expect PASS
5. Commit: `git commit -m "feat(interactive): add Start Codex menu option"`

**Validation (tester):**
- Full suite passes
- Menu helper covers gated options (active / none / stale)
- No regressions to render_main_screen / other menu tests
- Lint + typecheck pass

**Phase 2 End Review:**
- `spec-reviewer` → Phase 2 goal met; CLI + interactive match Expected State
- `code-quality-reviewer` → code style, standards, no placeholders
- Fix findings: `implementer` + `tester`, max 2 iterations, then move to next phase
- **Gate:** pass to next phase after max 2 fix iterations

### Phase 3: Docs polish

**Goal:** Document the dual start targets and auto-sync behavior in README.

#### Task 3.1: README `start` documentation [S after Task 2.1]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Update Commands table for bare `start`, `start claude`, and `start codex`. Add a short note covering auto-sync, active profile as `model_provider`, `opus` as `--model`, and API key staying in Codex config. Clarify that `show-command` remains Claude-only (`start` / `start claude`). Update the Quick Start menu sketch to include `Start Codex`. Do not rewrite the Sync section (still accurate); cross-link if useful.

**Files:**
- Modify: `README.md`

**Code Preview:** *(docs only — no code interface)*

Commands table rows:

| Command | Description |
| --- | --- |
| `cc-profile start` | Launch Claude Code with the active profile (alias of `start claude`) |
| `cc-profile start claude` | Launch Claude Code with the active profile |
| `cc-profile start codex` | Sync providers into Codex config, then launch Codex with the active profile as `model_provider` and its Opus model |
| `cc-profile show-command` | Print the exact Claude shell command that `start` / `start claude` would run |

**Steps (run by implementer):**

1. Write/adjust any doc-adjacent checks if present (none expected); otherwise skip to edit
2. N/A for pure docs RED phase — implementer verifies links/anchors still match ToC
3. Edit README Commands table + short paragraph on codex start behavior + Quick Start menu line + show-command Claude-only wording
4. Sanity: `rg "start codex" README.md` finds the new docs; ToC unchanged unless new section added
5. Commit: `git commit -m "docs: document start claude|codex targets"`

**Validation (tester):**
- README Commands table lists all three start forms
- Auto-sync + opus mapping described accurately
- `show-command` documented as Claude-only
- Quick Start menu sketch includes Start Codex
- Full test suite still passes (docs-only change)
- Lint + typecheck pass

**Phase 3 End Review:**
- `spec-reviewer` → all Success Criteria met; implementation matches plan; final gate
- `code-quality-reviewer` → no placeholders, standards clean across full diff
- Fix findings: `implementer` + `tester`, max 2 iterations
- **Gate:** plan complete after max 2 fix iterations
