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
        ├─ 1. resolve active profile (Err if missing)
        ├─ 2. sync_codex::sync(config, codex_config_path())
        │       (all profiles; reserved skipped; 0o600/0o700)
        ├─ 3. build_codex_command_spec
        │       program = CC_PROFILE_CODEX_BIN | "codex"
        │       args    = ["-c", "model_provider=\"<active>\"",
        │                  "--model", "<opus>"]
        │       envs    = {}   (key lives in Codex config, not argv/env)
        └─ 4. exec_command_spec (reuse existing launcher)

interactive menu: "Start Claude" + "Start Codex" (both gated on active profile)
```

## Testing

- **Framework:** Rust built-in `#[test]` + `assert_cmd` / `assert_fs` / `predicates` for integration
- **TDD cycle:** failing test → run (FAIL) → implement → run (PASS) → commit
- **Coverage target:** every new public/pub(crate) function has unit tests; CLI path has integration coverage via a codex shim (mirrors Claude shim)
- **Test files:**
  - `src/services/launch.rs` (unit tests for codex spec + start_codex preconditions)
  - `src/cli.rs` (clap parse coverage if needed; prefer integration)
  - `src/interactive.rs` (menu summary pure helpers if extracted)
  - `tests/integration/launch.rs` (extend: `start` still Claude; `start codex` argv + provider written)
  - `tests/integration/common.rs` (add `test_codex_shim()`)
  - `tests/fixtures/cc-profile-test-codex.rs` (argv-capturing shim)

## Success Criteria

- [ ] `cc-profile start` and `cc-profile start claude` still launch Claude with existing env/args behavior (no regression)
- [ ] `cc-profile start codex` auto-syncs providers into Codex config, then execs `codex -c model_provider="<active>" --model "<opus>"`
- [ ] Missing / stale active profile returns a clear error before any Codex process is started
- [ ] Reserved active-profile name (e.g. `openai`) fails with a clear error after sync (provider was skipped; cannot launch)
- [ ] API key never appears on the Codex argv; only in the 0o600 config `http_headers`
- [ ] Interactive menu offers "Start Codex" when an active profile exists
- [ ] README documents `start [claude|codex]` and the opus → model mapping
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

**Scope:** Add a pure builder that resolves the active profile and returns a `CommandSpec` for Codex. Program comes from `CC_PROFILE_CODEX_BIN` (or an explicit override helper, mirroring Claude) defaulting to `"codex"`. Args are exactly `["-c", "model_provider=\"<name>\"", "--model", "<opus>"]` — the provider value is double-quoted so Codex's TOML parser treats it as a string even for names like `true` or `123`. Envs are empty. Does NOT write files, does NOT exec, does NOT call sync.

**Files:**
- Modify: `src/services/launch.rs`
- Test: unit tests in `src/services/launch.rs` `#[cfg(test)]`

**Code Preview:**

```rust
// crucial: quoted provider value + opus model; empty envs; program override
pub(crate) fn build_codex_command_spec_with_program(
    config: &Config,
    program_override: Option<String>,
) -> Result<CommandSpec> {
    let (name, profile) = active_profile(config)?; // shared helper or inline
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
3. Implement minimum code: public `build_codex_command_spec` reading `CC_PROFILE_CODEX_BIN`, plus `build_codex_command_spec_with_program`; reuse active-profile resolution pattern from `build_command_spec_with_program` (extract a private helper only if it reduces duplication cleanly)
4. Run tests — expect PASS
5. Commit: `git commit -m "feat(launch): build Codex CommandSpec from active profile opus"`

**Validation (tester):**
- Full test suite passes
- All behaviors in Scope are covered by tests
- No regressions on existing Claude launch tests
- Lint + typecheck pass (`cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`)

#### Task 1.2: `start_codex` auto-sync + exec [S after Task 1.1]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Add `start_codex(config)` that (1) validates active profile exists, (2) rejects reserved active profile names with a clear error (cannot select a provider that sync skipped), (3) calls `sync_codex::sync(&config, &codex_config_path()?)`, (4) builds the Codex spec, (5) reuses the existing launcher/`exec_command_spec` path. Injectable launcher (mirror `start_claude_with_launcher`) so unit tests never real-exec. Does NOT change Claude start. Does NOT print CLI-facing messages (caller owns UX).

**Files:**
- Modify: `src/services/launch.rs`
- Test: unit tests in `src/services/launch.rs` `#[cfg(test)]` (tempdir + `CODEX_HOME` or explicit path injection)

**Code Preview:**

```rust
// crucial: sync-before-exec order; reserved active name fails closed
pub fn start_codex(config: &Config) -> Result<()> {
    start_codex_with_launcher(config, exec_command_spec)
}

fn start_codex_with_launcher<F>(config: &Config, launch: F) -> Result<()>
where
    F: FnOnce(&CommandSpec) -> Result<()>,
{
    let (name, _) = resolve_active_profile(config)?;
    if sync_codex::is_reserved_provider_id(&name) {
        bail!("Cannot start Codex: profile '{name}' is a reserved Codex provider id");
    }
    let path = sync_codex::codex_config_path()?;
    let _skipped = sync_codex::sync(config, &path)?;
    let spec = build_codex_command_spec(config)?;
    launch(&spec)
}
```

**Note:** `is_reserved_provider_id` is currently `pub(crate)` in `sync_codex.rs` — keep that visibility; `launch` is the same crate. If tests need a path override for sync, prefer injecting via `CODEX_HOME` behind the existing `ENV_LOCK` pattern in `sync_codex` tests, or add a `start_codex_with_paths_and_launcher` test-only seam — do **not** expand the public API without need.

**Steps (run by implementer):**

1. Write failing tests covering:
   - Happy path with injectable launcher: sync writes provider block under temp `CODEX_HOME`, launcher receives expected `CommandSpec`
   - Missing active profile fails before launch (launcher not called)
   - Reserved active profile (`openai`) fails with clear message; launcher not called
   - Sync error (e.g. invalid existing TOML under `CODEX_HOME`) propagates; launcher not called
2. Run tests — expect FAIL
3. Implement minimum code; make `is_reserved_provider_id` reachable if needed (`pub(crate)` already is)
4. Run tests — expect PASS
5. Commit: `git commit -m "feat(launch): start Codex after syncing active profile provider"`

**Validation (tester):**
- Full test suite passes
- Auto-sync side effect verified (provider block present on disk after start_codex unit test)
- Reserved / missing active-profile paths never call launcher
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

**Scope:** Change `Command::Start` to carry an optional nested target, defaulting bare `start` to Claude. Dispatch `Claude` → existing `start_command` / `launch::start_claude`; `Codex` → new handler calling `launch::start_codex`. Preserve exit codes and error messages from the service layer. Add integration coverage with a Codex argv-capturing shim (mirror Claude). Does NOT change other subcommands.

**Files:**
- Modify: `src/cli.rs`
- Create: `tests/fixtures/cc-profile-test-codex.rs`
- Modify: `tests/integration/common.rs` (add `test_codex_shim()`)
- Modify: `tests/integration/launch.rs` (Claude regression + new codex test)
- Optionally extend: `tests/integration/cli.rs` help text assertions

**Code Preview:**

```rust
// crucial: optional nested target keeps bare `start` working
Start {
    #[command(subcommand)]
    target: Option<StartTarget>,
},

#[derive(Debug, Subcommand)]
pub enum StartTarget {
    Claude,
    Codex,
}

// dispatch
Some(Command::Start { target: None | Some(StartTarget::Claude) }) => start_command(&repository),
Some(Command::Start { target: Some(StartTarget::Codex) }) => start_codex_command(&repository),
```

**Steps (run by implementer):**

1. Write failing tests covering:
   - Integration: existing `start` Claude shim test still passes
   - Integration: `start claude` same as bare `start` (Claude envs)
   - Integration: `start codex` with `HOME` + `CODEX_HOME` + `CC_PROFILE_CODEX_BIN` shim:
     - exit 0
     - shim output contains `-c` and `model_provider="profile-a"` and `--model` + opus value
     - `$CODEX_HOME/config.toml` contains `[model_providers.profile-a]` and `Bearer …`
     - shim argv does **not** contain the api key
   - Integration: `start codex` with no active profile → failure, stderr contains `No active profile`
   - Help lists `claude` / `codex` under `start` (optional but preferred)
2. Run tests — expect FAIL
3. Implement clap enum + dispatch + `start_codex_command`; add codex shim fixture + `test_codex_shim()` (copy Claude fixture, capture `args` only — envs optional)
4. Run tests — expect PASS
5. Commit: `git commit -m "feat(cli): add start claude|codex subcommand targets"`

**Validation (tester):**
- Full suite passes including new integration tests
- Bare `start` regression green
- Codex path auto-sync + argv contract verified
- Lint + typecheck pass

#### Task 2.2: Interactive "Start Codex" menu option [P with Task 2.1 after Task 1.2]

**Note on parallelism:** Task 2.2 only needs Task 1.2's `launch::start_codex`. It can run in parallel with Task 2.1 if implementers touch different files carefully (`interactive.rs` vs `cli.rs`/tests). If merge conflict risk is high, run sequentially after 2.1. Prefer **parallel** when both start from a clean tree post-1.2.

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** When an active profile exists, offer `"Start Codex"` alongside `"Start Claude"`. Selecting it calls `launch::start_codex(&config)`. Keep `"Start Claude"` behavior unchanged. Extract a pure helper only if needed for testability (e.g. menu option list builder); do not over-abstract.

**Files:**
- Modify: `src/interactive.rs`
- Test: unit tests in `src/interactive.rs` if a pure menu-options helper is extracted; otherwise rely on compile + manual smoke note

**Code Preview:**

```rust
// crucial: gate both start options on active_profile_exists
if active_profile_exists(&config) {
    options.push("Start Claude");
    options.push("Start Codex");
}
// match arms:
"Start Claude" => launch::start_claude(&config)?,
"Start Codex" => launch::start_codex(&config)?,
```

**Steps (run by implementer):**

1. Write failing tests covering:
   - If extracting `main_menu_options(config) -> Vec<&'static str>` (recommended for TDD): with active profile includes both Start entries; without active profile includes neither; always includes Quit
   - If not extracting: document why and rely on match exhaustiveness + compile
2. Run tests — expect FAIL (if helper extracted)
3. Implement menu option + match arm
4. Run tests — expect PASS
5. Commit: `git commit -m "feat(interactive): add Start Codex menu option"`

**Validation (tester):**
- Full suite passes
- Menu helper (if any) covers gated options
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

**Scope:** Update Commands table and add a short note under Commands (or a brief subsection) covering: bare `start` = Claude; `start claude` / `start codex`; codex auto-syncs providers then launches with active profile as provider and `opus` as `--model`; API key stays in Codex config. Optionally update Quick Start menu sketch to show `Start Codex`. Do not rewrite the Sync section (still accurate); cross-link if useful.

**Files:**
- Modify: `README.md`

**Code Preview:** *(docs only — no code interface)*

Commands table rows:

| Command | Description |
| --- | --- |
| `cc-profile start` | Launch Claude Code with the active profile (alias of `start claude`) |
| `cc-profile start claude` | Launch Claude Code with the active profile |
| `cc-profile start codex` | Sync providers into Codex config, then launch Codex with the active profile as `model_provider` and its Opus model |

**Steps (run by implementer):**

1. Write/adjust any doc-adjacent checks if present (none expected); otherwise skip to edit
2. N/A for pure docs RED phase — implementer verifies links/anchors still match ToC
3. Edit README Commands table + short paragraph on codex start behavior
4. Sanity: `rg "start codex" README.md` finds the new docs; ToC unchanged unless new section added
5. Commit: `git commit -m "docs: document start claude|codex targets"`

**Validation (tester):**
- README Commands table lists all three start forms
- Auto-sync + opus mapping described accurately
- Full test suite still passes (docs-only change)
- Lint + typecheck pass

**Phase 3 End Review:**
- `spec-reviewer` → all Success Criteria met; implementation matches plan; final gate
- `code-quality-reviewer` → no placeholders, standards clean across full diff
- Fix findings: `implementer` + `tester`, max 2 iterations
- **Gate:** plan complete after max 2 fix iterations
