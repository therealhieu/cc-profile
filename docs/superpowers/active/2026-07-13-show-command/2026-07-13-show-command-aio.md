# show-command — All-in-One Plan

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

Users can activate a profile and run `cc-profile start`, but there is no way to see the exact shell command that `start` executes. This blocks several concrete workflows:

- **Issue 1: No way to inspect the resolved command without launching Claude.** — *Evidence:* The only code path that assembles `ANTHROPIC_*` env vars + `claude` flags is `launch::build_command_spec` (`src/services/launch.rs:26`), and it is only ever consumed by `start_claude`, which immediately `exec`s the process (`src/services/launch.rs:115`). There is no read-only surface. — *Solution:* Add a `cc-profile show-command` subcommand that builds the same `CommandSpec` and prints it instead of executing it (Task 2.1).

- **Issue 2: Users cannot copy-paste the launch command into scripts, CI, or a debugging shell.** — *Evidence:* `cc-profile show` (`src/cli.rs:238`) prints the TOML config, not a runnable command line; a user wanting `ANTHROPIC_BASE_URL=… ANTHROPIC_API_KEY=… claude …` must hand-assemble it from six config fields plus the skip-permissions flag. Values like endpoints/keys may contain characters that need shell quoting. — *Solution:* Render a single copy-pasteable line with POSIX single-quote shell-escaping on every value (Task 1.1), so it is safe to paste as-is.

- **Issue 3: A hand-assembled command drifts from what `start` actually runs.** — *Evidence:* `build_command_spec` applies profile `ANTHROPIC_*` values *after* global `config.envs` so the profile wins (`src/services/launch.rs:44-56`), and honors `CC_PROFILE_CLAUDE_BIN` for the program name (`src/services/launch.rs:27`). A user reconstructing the command by hand would miss this precedence and the binary override. — *Solution:* `show-command` reuses `build_command_spec` verbatim, so the printed command is guaranteed identical to what `start` launches (Task 2.1).

## Goal

Add `cc-profile show-command`, which prints the exact, copy-pasteable shell command (`ANTHROPIC_* … claude [--flags]`) that `cc-profile start` would execute for the active profile.

## Non-Goals

- Masking or redacting the API key. Decision confirmed with the user: reveal it, consistent with `cc-profile show` and the interactive main screen, which already print it unmasked.
- Adding the command to the interactive menu. This plan ships the non-interactive subcommand only.
- Executing the command or copying it to the clipboard. Output goes to stdout only.
- Any new config fields, env vars, or flags beyond what `build_command_spec` already emits.

## Current State

```
cc-profile start
      │
      ▼
launch::start_claude(&config)
      │
      ▼
build_command_spec(config) ── reads CC_PROFILE_CLAUDE_BIN
      │                         merges config.envs + profile ANTHROPIC_* + args
      ▼
CommandSpec { program, args, envs }
      │
      ▼
exec_command_spec  ── replaces process, NEVER returns   ← no read-only surface

cc-profile show ──► prints TOML config (not a runnable command)
```

## Expected State

```
cc-profile start ─────────────┐
cc-profile show-command ──┐    │
                          ▼    ▼
              build_command_spec(config)   ← single source of truth (unchanged)
                          │
              ┌───────────┴───────────┐
              ▼                        ▼
   render_command_line(&spec)    exec_command_spec(&spec)
   (pure, shell-quoted String)   (replaces process)
              │
              ▼
   println! to stdout:
   ANTHROPIC_API_KEY='sk-ant-secret' ANTHROPIC_BASE_URL='https://api.anthropic.com' \
   ANTHROPIC_DEFAULT_FABLE_MODEL='claude-fable-5' … claude --dangerously-skip-permissions
```

## Testing

- **Framework:** Rust built-in (`#[test]`), `cargo test`. Unit tests inline in `src/services/launch.rs`; integration tests via `assert_cmd` + `assert_fs` + `predicates` in `tests/integration/cli.rs` (a module of the `tests/integration/main.rs` harness — NOT a standalone `tests/cli.rs` binary, which would lack access to the `write_config` helper).
- **TDD cycle:** failing test → `cargo test` (FAIL) → implement → `cargo test` (PASS) → commit.
- **Coverage target:** Every new function (`render_command_line`, `shell_quote`) has direct unit tests; the subcommand has one integration test covering success and one covering the no-active-profile error.
- **Test files:** `src/services/launch.rs` (unit), `tests/integration/cli.rs` (integration).

## Success Criteria

- [ ] `cc-profile show-command` prints a single line: sorted `KEY='value'` env assignments, then the program, then args.
- [ ] Every env value and arg is POSIX single-quote escaped so the line is safe to paste into `sh`/`bash`/`zsh`.
- [ ] The printed env vars, program, and args exactly match what `build_command_spec` produces (same precedence, same `CC_PROFILE_CLAUDE_BIN` handling).
- [ ] With no active profile, the command exits non-zero with the existing `No active profile is set` message (no new error text invented).
- [ ] `cc-profile --help` lists `show-command`.
- [ ] All tests pass.
- [ ] Lint (`cargo clippy`) and format (`cargo fmt --check`) pass.
- [ ] No placeholders remain.

## Project Standards

- Follow repo conventions in `AGENTS.md` and the existing `src/services/launch.rs` / `src/cli.rs` style: `//!` module docs, `///` doc comments with `# Errors` sections on fallible public fns, `anyhow::Result` returns.
- Match the CI gate in `scripts/ci.sh` (fmt, clippy, test). Verify with it before completion.
- Mirror the existing test idioms: inline `#[cfg(test)] mod tests` with a `sample`/`active_config` helper for unit tests; `Command::cargo_bin("cc-profile").env("HOME", temp.path())` for integration tests (see `show_prints_config_with_unmasked_api_key` and `start_launches_claude_with_profile_envs_and_configured_args`).
- Clap derives kebab-case subcommand names automatically, so a `ShowCommand` variant becomes `show-command` — matches the existing `Command` enum pattern in `src/cli.rs:24`.

## Implementation Plan

**Hierarchy:** `Phase → Task → Step`

**Parallelism key:** `[P]` = parallel with siblings, `[S]` = sequential, `[S after Task X.Y]` = blocks on X.Y.

### Phase 1: Rendering core

**Goal:** Deliver a pure, unit-tested function that turns a `CommandSpec` into a shell-safe, copy-pasteable command line, plus its quoting helper.

#### Task 1.1: `render_command_line` + `shell_quote` in launch.rs [S]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Add two functions to `src/services/launch.rs`. `shell_quote` wraps a string in single quotes, escaping any embedded single quote using the POSIX `'\''` idiom, so arbitrary endpoint/key/model/arg values are safe to paste into a shell. `render_command_line` takes a `&CommandSpec` and returns a `String`: env assignments in `spec.envs` iteration order (already BTreeMap-sorted → deterministic) formatted as `KEY=<quoted value>`, followed by the (quoted) `program`, followed by each (quoted) arg, all space-joined. This task is pure logic only — no I/O, no CLI wiring, no printing. It does NOT touch `build_command_spec`, `start_claude`, or `exec_command_spec`.

**Files:**
- Modify: `src/services/launch.rs`
- Test: `src/services/launch.rs` (inline `#[cfg(test)] mod tests`)

**Code Preview:** *(crucial parts only — the quoting contract and assembly order)*

```rust
// crucial: POSIX single-quote escaping — the ONLY safe way to quote arbitrary values.
// A literal ' becomes '\'' : close quote, escaped quote, reopen quote.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// Renders `spec` as a single copy-pasteable shell command line:
/// `KEY='v' KEY2='v2' <program> <arg>...`, envs in `spec.envs` (sorted) order.
pub fn render_command_line(spec: &CommandSpec) -> String {
    let mut parts: Vec<String> = spec
        .envs
        .iter()
        .map(|(k, v)| format!("{k}={}", shell_quote(v)))
        .collect();
    parts.push(shell_quote(&spec.program));
    parts.extend(spec.args.iter().map(|a| shell_quote(a)));
    parts.join(" ")
}
```

**Steps (run by implementer):**

1. Write failing unit tests covering:
   - `shell_quote` wraps a plain value in single quotes (`sk-ant-secret` → `'sk-ant-secret'`).
   - `shell_quote` escapes an embedded single quote using the `'\''` idiom.
   - `render_command_line` emits sorted `KEY='value'` pairs, then program, then args, for an `active_config`-style spec (reuse/adapt the existing `active_config` helper) — assert the full expected string including `--dangerously-skip-permissions` when enabled.
   - `render_command_line` emits no trailing args when the args vec is empty.
2. Run `cargo test` — expect FAIL (functions do not exist).
3. Implement `shell_quote` and `render_command_line` per Code Preview.
4. Run `cargo test` — expect PASS.
5. Commit: `git commit -m "feat(launch): render CommandSpec as shell-quoted command line"`

**Validation (tester):**
- `cargo test` full suite passes.
- Tests cover: plain quoting, embedded-quote escaping, full-line assembly with args, empty-args case.
- No regressions in existing launch tests.
- `cargo clippy` + `cargo fmt --check` pass.

**Phase 1 End Review:**
- `spec-reviewer` → Phase 1 goal met (pure render fn exists, unit-tested); matches plan; no scope drift into CLI wiring.
- `code-quality-reviewer` → doc comments present, POSIX quoting correct, standards met, no placeholders.
- Fix findings: `implementer` + `tester`, max 2 iterations, then move to next phase.
- **Gate:** pass to Phase 2 after max 2 fix iterations.

### Phase 2: CLI wiring + docs

**Goal:** Expose the renderer as `cc-profile show-command`, verified end-to-end, and document it.

#### Task 2.1: `show-command` subcommand + handler [S after Task 1.1]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Wire the renderer into the CLI in `src/cli.rs`. Add a `ShowCommand` variant to the `Command` enum (clap renders it as `show-command`). Add a handler `show_command(&repository)` that loads config, calls `launch::build_command_spec(&config)?`, and `println!`s `launch::render_command_line(&spec)`. Propagate the existing `build_command_spec` errors unchanged (no active profile → `No active profile is set`). Add the match arm in `run()`. This task does NOT modify rendering logic or `build_command_spec`.

**Files:**
- Modify: `src/cli.rs`
- Test: `tests/integration/cli.rs` (integration; add `#[test]` fns to the existing module alongside `show_prints_config_with_unmasked_api_key`)

**Code Preview:** *(crucial parts only — the handler contract; enum variant and match arm are mechanical)*

```rust
// crucial: reuse build_command_spec so output is guaranteed identical to `start`.
fn show_command(repository: &ConfigRepository) -> Result<()> {
    let config = repository.load()?;
    let spec = launch::build_command_spec(&config)?;
    println!("{}", launch::render_command_line(&spec));
    Ok(())
}
```

**Steps (run by implementer):**

1. Write failing integration tests in `tests/integration/cli.rs` (add `#[test]` fns to the existing module; reuse the `write_config` helper):
   - `show_command_prints_runnable_command_line`: run `show-command` with `HOME` pointed at a temp config (active `profile-a`), assert stdout contains `ANTHROPIC_BASE_URL='https://api.anthropic.com'`, `ANTHROPIC_API_KEY='sk-ant-secret'`, `ANTHROPIC_DEFAULT_FABLE_MODEL='claude-fable-5'`, and ends with `claude` (no skip-permissions flag in that fixture).
   - `show_command_errors_without_active_profile`: do NOT use `write_config` (it always sets `active_profile = "profile-a"`). Point `HOME` at an empty temp dir so no config file exists — `load()` yields the default with `active_profile = None`, and `build_command_spec` bails. Assert failure + stderr contains `No active profile is set`.
   - Extend the existing `help_lists_core_commands` test (`tests/integration/cli.rs:48`) to also assert `--help` lists `show-command` (mandatory — matches Success Criteria; do not duplicate the help test).
2. Run `cargo test` — expect FAIL (subcommand unknown).
3. Add the `ShowCommand` enum variant, the `Some(Command::ShowCommand) => show_command(&repository)` arm, and the `show_command` handler.
4. Run `cargo test` — expect PASS.
5. Commit: `git commit -m "feat(cli): add show-command subcommand"`

**Validation (tester):**
- `cargo test` full suite passes (unit + integration).
- Success path asserts sorted, single-quoted env assignments and the `claude` program.
- Error path asserts the reused `No active profile is set` message and non-zero exit.
- `--help` output includes `show-command`.
- `cargo clippy` + `cargo fmt --check` pass.

#### Task 2.2: Document `show-command` in README [P with Task 2.1]

**Subagent:** `implementer` (TDD-exempt: docs) → `tester` (validate)

**Scope:** Add one row to the Commands table in `README.md` describing `cc-profile show-command`, placed immediately after the `cc-profile show` row. One line, matching the table's existing style. No code changes. TDD does not apply to a docs-only change; the implementer edits the table directly.

**Files:**
- Modify: `README.md`

**Code Preview:** *(the exact table row to add)*

```markdown
| `cc-profile show-command` | Print the exact shell command (`ANTHROPIC_* … claude`) that `start` would run for the active profile |
```

**Steps (run by implementer):**

1. Locate the Commands table row for `cc-profile show` in `README.md`.
2. Insert the new `show-command` row immediately after it.
3. Verify table renders (pipe alignment consistent with neighbors).
4. Commit: `git commit -m "docs: document show-command subcommand"`

**Validation (tester):**
- README Commands table contains the `show-command` row.
- Description matches actual behavior (prints command, does not execute).
- No broken markdown table formatting.

**Phase 2 End Review:**
- `spec-reviewer` → all Success Criteria met; `show-command` output matches `start`; docs match behavior; no scope drift.
- `code-quality-reviewer` → clap variant/handler idiomatic, doc comment on handler if warranted, standards met, no placeholders, `scripts/ci.sh` clean.
- Fix findings: `implementer` + `tester`, max 2 iterations, then done.
- **Gate:** last phase — this review is the final gate.
