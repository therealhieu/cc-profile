# show-command-targets — All-in-One Plan

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

- **Issue 1: `show-command` is Claude-only while `start` is dual-target.** — *Evidence:* `Command::ShowCommand` has no nested target (`src/cli.rs:35`), and `show_command` always calls `launch::build_command_spec` (`src/cli.rs:386-390`). The README Commands table describes bare show-command as Claude (`README.md:149`), and the narrative still says “`show-command` remains Claude-only” (`README.md:156`). Users can `start codex` but cannot inspect the resolved Codex line. — *Solution:* Task 1.1 nests optional `StartTarget` under `ShowCommand`, defaulting bare `show-command` to Claude.

- **Issue 2: No read-only path for the Codex `CommandSpec`.** — *Evidence:* `build_codex_command_spec` already exists (`src/services/launch.rs:82-113`) and `render_command_line` already renders any `CommandSpec` (`src/services/launch.rs:160-171`), but only `start_codex` consumes the builder — and it also syncs + execs. There is no print-only Codex surface. — *Solution:* Task 1.1 routes `show-command codex` through reserved-id check + `build_codex_command_spec` + `render_command_line` with no sync and no exec.

- **Issue 3: Docs and help still describe Claude-only show-command.** — *Evidence:* Commands table row (`README.md:149`) plus narrative (`README.md:156`) claim Claude-only; that was an intentional non-goal of `start-codex`. — *Solution:* Task 2.1 updates README to document `show-command [claude|codex]`, print-only Codex semantics, and reserved-id failure.

## Goal

Deliver `cc-profile show-command [claude|codex]` that prints the exact copy-pasteable shell line for the active profile’s Claude or Codex launch, with bare `show-command` remaining Claude and Codex print never writing Codex config.

## Non-Goals

- Syncing Codex providers during `show-command codex` (print only; user runs `sync codex` / `start codex` for writes).
- Interactive menu entry for show-command.
- Redacting or masking API keys (same as current Claude `show-command` / `show`).
- Renaming `StartTarget` → `LaunchTarget` (reuse existing enum).
- Extracting a shared resolve/reserved helper or making `resolve_active_profile` public (inline the reserved gate in `show_command` only).
- Changing `build_*_command_spec` argv/env semantics, context-marker mapping, or `render_command_line` quoting.
- Clipboard / execute-from-show-command behavior.
- Making bare `show-command` require an explicit target.
- Inventing show-specific reserved-id wording (reuse the start message for parity).

## Current State

```
cc-profile start [claude|codex]
        │
        ├─ None | Claude → build_command_spec → exec
        └─ Codex         → resolve → reject reserved
                           → sync → build_codex_command_spec → exec

cc-profile show-command
        │
        ▼
build_command_spec (Claude only)
        │
        ▼
render_command_line → stdout

README: "show-command remains Claude-only"
```

## Expected State

```
cc-profile show-command              ─┐
cc-profile show-command claude       ─┴─► build_command_spec
                                           → render_command_line → stdout

cc-profile show-command codex
        │
        ├─ resolve active profile (missing/stale → same errors as builders)
        ├─ reject reserved id (same message as start codex)
        │     fail BEFORE render; no Codex config write
        ├─ build_codex_command_spec   (NO sync)
        └─ render_command_line → stdout
             shape: CC_PROFILE_API_KEY=… codex -c model_provider="…" --model <bare-opus>
                    [+ -c model_context_window=… when opus marker is known]
                    (exact quoting comes from render_command_line / shell_quote)

start [claude|codex] unchanged (still syncs on codex start)
```

## Testing

- **Framework:** Rust built-in `#[test]` + `assert_cmd` / `assert_fs` / `predicates` for integration
- **TDD cycle:** failing test → run (FAIL) → implement → run (PASS) → commit
- **Coverage target:** CLI success paths for bare/claude/codex; reserved-id and no-active-profile errors (bare + codex); no Codex config write on show-command codex; help lists nested targets; README wording
- **Test files:**
  - `tests/integration/cli.rs` (extend show-command cases; keep using local `write_config` / hand-written fixtures in that file)
  - `src/cli.rs` only if a pure helper is extracted (prefer integration; plan chooses **no** new helper)
  - Unit coverage for builders/render already lives in `src/services/launch.rs` — do not re-test those contracts

## Success Criteria

- [ ] `cc-profile show-command` and `cc-profile show-command claude` print the Claude shell line (existing behavior preserved)
- [ ] `cc-profile show-command codex` prints the Codex shell line from `build_codex_command_spec` via `render_command_line` (includes `CC_PROFILE_API_KEY`, program, `model_provider=…`, `--model`; inherits builder context-window semantics already unit-tested in `launch.rs`)
- [ ] `show-command codex` does **not** create or modify Codex config under `$CODEX_HOME` (integration test sets `CODEX_HOME` to a temp dir and asserts no `config.toml`)
- [ ] Reserved active profile that exists in config (e.g. `openai`) fails with `Cannot start Codex: profile '…' is a reserved Codex provider id` and does not print a command line
- [ ] No active profile fails with `No active profile is set` for bare `show-command` and `show-command codex`
- [ ] `cc-profile show-command --help` surfaces nested `claude` / `codex` targets
- [ ] README documents `show-command [claude|codex]`, print-only Codex, and reserved-id failure; removes “Claude-only” wording
- [ ] All tests pass
- [ ] `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass
- [ ] No placeholders remain

## Project Standards

- Follow `AGENTS.md` → `docs/specs` (Rust style, testing, development, git).
- Match existing CLI patterns in `src/cli.rs`: optional nested `StartTarget` like `Command::Start`, dispatch in `run()`, thin handler functions.
- Reuse `launch::build_command_spec`, `launch::build_codex_command_spec`, `launch::render_command_line`, and `sync_codex::is_reserved_provider_id` — do not fork command construction.
- Integration tests live in `tests/integration/cli.rs` using `Command::cargo_bin("cc-profile").env("HOME", temp.path())` and that file’s existing `write_config` helper / hand-written TOML fixtures (same idiom as current `show_command_*` tests). For Codex no-write assertions, also set `CODEX_HOME` on the child (mirror `tests/integration/launch.rs` start-codex tests).
- Verify with `scripts/ci.sh` (or equivalent fmt/clippy/test gate) before completion.
- Commit messages: conventional (`feat:`, `test:`, `docs:`).

## Implementation Plan

**Hierarchy:** `Phase → Task → Step`

**Parallelism key:** `[P]` = parallel with siblings, `[S]` = sequential, `[S after Task X.Y]` = blocks on X.Y.

### Phase 1: CLI dual-target show-command

**Goal:** Wire nested targets so bare/claude/codex print the correct launch line without Codex sync.

#### Task 1.1: Nested `ShowCommand` target + print-only handlers [S]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Extend `Command::ShowCommand` with optional nested `StartTarget` (reuse the existing enum). Dispatch bare/`claude` through the current Claude builder path and `codex` through an **inline** reserved-id check (same order and messages as `start_codex_with_path_and_launcher`) then `build_codex_command_spec` + `render_command_line`. Do **not** call `sync_codex::sync`, `start_codex`, or any launcher. Do **not** extract a shared helper or make `resolve_active_profile` public. Do not change interactive menu. Do not alter builder/render semantics.

**Files:**
- Edit: `src/cli.rs`
- Test: `tests/integration/cli.rs`

**Code Preview:** *(crucial: clap shape + full codex branch without sync)*

```rust
// crucial: mirror Start; reuse StartTarget; inline reserved gate (no shared helper)
ShowCommand {
    #[command(subcommand)]
    target: Option<StartTarget>,
},
// run(): Some(Command::ShowCommand { target }) => show_command(&repository, target),

fn show_command(repository: &ConfigRepository, target: Option<StartTarget>) -> Result<()> {
    let config = repository.load()?;
    let spec = match target {
        None | Some(StartTarget::Claude) => launch::build_command_spec(&config)?,
        Some(StartTarget::Codex) => {
            // same order as start_codex_with_path_and_launcher — no sync
            let Some(name) = config.active_profile.as_deref() else {
                anyhow::bail!("No active profile is set");
            };
            if !config.profiles.contains_key(name) {
                anyhow::bail!("Active profile '{name}' does not exist");
            }
            if sync_codex::is_reserved_provider_id(name) {
                anyhow::bail!(
                    "Cannot start Codex: profile '{name}' is a reserved Codex provider id"
                );
            }
            launch::build_codex_command_spec(&config)?
        }
    };
    println!("{}", launch::render_command_line(&spec));
    Ok(())
}
```

**Steps (run by implementer):**

1. Write failing tests in `tests/integration/cli.rs` covering:
   - Existing bare `show-command` still prints Claude env/program (keep/adjust current test).
   - `show-command claude` prints the same Claude line shape (`ANTHROPIC_*`, `claude`).
   - `show-command codex` stdout contains `CC_PROFILE_API_KEY=`, `codex`, `model_provider=`, and the active profile’s opus model (use default `write_config` fixture).
   - `show-command codex` no-write: set `.env("CODEX_HOME", codex_home.path())` on a fresh temp dir and assert `codex_home.child("config.toml")` does not exist after success.
   - Reserved id: hand-write a config with active profile `openai` that **exists** in `[profiles.openai]` (pattern from `tests/integration/launch.rs` reserved-start fixture / `tests/integration/sync.rs`); `show-command codex` fails with `Cannot start Codex: profile 'openai' is a reserved Codex provider id`; stdout has no `codex` command line.
   - No active profile: bare `show-command` and `show-command codex` both fail with `No active profile is set`.
   - `show-command --help` stdout contains `claude` and `codex`.
2. Run tests — expect FAIL (clap rejects nested target / codex not implemented).
3. Implement minimum code in `src/cli.rs`:
   - Change `ShowCommand` to `ShowCommand { target: Option<StartTarget> }`.
   - Update `run()` arm to `Some(Command::ShowCommand { target }) => show_command(&repository, target)`.
   - Implement `show_command` per Code Preview: Claude path unchanged; Codex path resolves active → rejects missing profile → reserved bail (exact start message) → `build_codex_command_spec` → `render_command_line`. No `sync_codex::sync`.
4. Run tests — expect PASS.
5. Commit: `git commit -m "feat: add show-command claude|codex targets"`

**Validation (tester):**
- Full test suite passes
- Cases covered: bare Claude, explicit claude, codex success, CODEX_HOME no-write, reserved fail, no-active bare + codex, show-command help targets
- No regressions on existing Claude `show-command`
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass
- Confirmed no Codex config write under the test `CODEX_HOME`

**Phase 1 End Review:**
- `spec-reviewer` → Phase 1 goal met; tasks match plan; Success Criteria progress
- `code-quality-reviewer` → code style, standards, no placeholders
- Fix findings: `implementer` + `tester`, max 2 iterations, then move to next phase
- **Gate:** pass to next phase after max 2 fix iterations

### Phase 2: Docs

**Goal:** Document dual-target show-command and remove Claude-only claims.

#### Task 2.1: README Commands table + narrative [S after Task 1.1]

**Subagent:** `implementer` (TDD-exempt: docs) → `tester` (validate)

**Scope:** Update `README.md` Commands table and the short paragraph after it so `show-command` documents bare/`claude`/`codex`, states Codex is print-only (no sync), and notes reserved-id failure. Remove “`show-command` remains Claude-only”. When rewriting that paragraph, do **not** restate obsolete Codex auth claims; only fix show-command wording. Do not rewrite unrelated sections (Sync body stays unless it wrongly claims show-command Claude-only).

**Files:**
- Edit: `README.md`
- Test: none (docs-only; verify with `rg` + full suite)

**Code Preview:** *(crucial: table rows + narrative intent)*

```markdown
| `cc-profile show-command` | Print the Claude shell command (`start` / `start claude`) |
| `cc-profile show-command claude` | Same as bare `show-command` |
| `cc-profile show-command codex` | Print the Codex shell command `start codex` would run (no sync; reserved active profile fails like start) |

# Replace the sentence ending with `show-command remains Claude-only`
# with: show-command mirrors start targets; codex path is print-only (no provider sync).
```

**Steps (run by implementer):**

1. Edit Commands table + narrative paragraph per Scope/Preview.
2. Verify with `rg -n "show-command remains Claude-only|show-command" README.md` that Claude-only claim is gone and new rows/targets are present.
3. Run full suite — expect PASS (docs-only change).
4. Commit: `git commit -m "docs: document show-command claude|codex"`

**Validation (tester):**
- README no longer says show-command is Claude-only
- Table/paragraph match implemented CLI behavior (nested targets, print-only codex, reserved fail)
- Full suite still passes
- Lint/format unchanged (docs-only)

**Phase 2 End Review:**
- `spec-reviewer` → Phase 2 goal met; docs match Success Criteria
- `code-quality-reviewer` → docs accurate, no placeholders, no scope creep
- Fix findings: `implementer` + `tester`, max 2 iterations
- **Gate:** last phase review is the final gate
