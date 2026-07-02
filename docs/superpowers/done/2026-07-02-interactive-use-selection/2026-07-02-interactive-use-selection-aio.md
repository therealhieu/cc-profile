# Interactive Use Selection — All-in-One Plan

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

- **Issue 1:** `cc-profile use` cannot be submitted without a profile name. — *Evidence:* `cargo run --quiet -- use` exits with clap code 2 and prints `the following required arguments were not provided: <PROFILE>`. — *Solution:* Task 1.1 changes the `Use` subcommand positional argument from required `String` to optional `Option<String>` and dispatches missing values to a dedicated selector.
- **Issue 2:** The existing fast path only supports users who already know and type the exact profile name. — *Evidence:* `src/cli.rs:86` sends `Some(Command::Use { profile })` directly to `use_profile(&repository, &profile)`. — *Solution:* Task 1.2 adds an interactive selection path that lists configured profile names, marks the active profile, and activates the selected name.
- **Issue 3:** Interactive fallback needs safe terminal behavior. — *Evidence:* `dialoguer::Select::interact()` does not support `Esc`/`q` cancellation, and non-TTY invocation should not hang waiting for a terminal selection. — *Solution:* Task 1.2 uses `interact_opt()` for cancellation and checks `stdin` before opening the selector.
- **Issue 4:** Regression coverage currently only proves `cc-profile use profile-b`. — *Evidence:* `tests/integration/cli.rs:78` covers the explicit-argument path but no missing-argument behavior. — *Solution:* Task 2.1 adds focused unit tests for selection helper behavior and integration coverage for direct use plus non-TTY missing-argument failure.

## Goal

Running `cc-profile use` in an interactive terminal opens a profile selector, while `cc-profile use <profile>` keeps its current direct activation behavior.

## Non-Goals

- Do not add a new `choose` command.
- Do not route `cc-profile use` into the full interactive profile management menu.
- Do not change profile storage format or validation rules.
- Do not change `cc-profile list`, `cc-profile start`, or the top-level interactive app flow.
- Do not add new dependencies; `dialoguer` is already available.

## Current State

```text
User
 ├─ cc-profile use profile-b
 │    ↓
 │  clap parses Command::Use { profile: String }
 │    ↓
 │  cli::use_profile(repository, "profile-b")
 │    ↓
 │  profiles::set_active_profile
 │    ↓
 │  config active_profile = "profile-b"
 │
 └─ cc-profile use
      ↓
    clap rejects missing <PROFILE>
      ↓
    exit 2 + usage error
```

## Expected State

```text
User
 ├─ cc-profile use profile-b
 │    ↓
 │  clap parses Command::Use { profile: Some("profile-b") }
 │    ↓
 │  cli::use_profile(repository, "profile-b")
 │    ↓
 │  unchanged direct activation
 │
 └─ cc-profile use
      ↓
    clap parses Command::Use { profile: None }
      ↓
    cli::use_profile_interactively(repository)
      ├─ load config
      ├─ no profiles → clear error, no config write
      ├─ non-TTY stdin → clear error, no config write
      └─ TTY stdin → dialoguer Select
             ├─ Enter on profile → use_profile(repository, selected)
             └─ Esc/q cancel → no config write
```

## Testing

- **Framework:** Rust unit tests plus integration tests via `assert_cmd`, `assert_fs`, and `predicates`.
- **TDD cycle:** failing test → run (FAIL) → implement → run (PASS) → commit.
- **Coverage target:** Preserve repository CI target; no explicit line target for this small CLI change beyond meaningful success and failure paths.
- **Test files:**
  - `src/cli.rs` for private helper unit tests.
  - `tests/integration/cli.rs` for CLI behavior tests.

## Success Criteria

- [ ] `cc-profile use profile-b` still sets `active_profile = "profile-b"` and prints the existing success message.
- [ ] `cc-profile use` parses successfully instead of failing clap's missing required argument check.
- [ ] In a TTY, `cc-profile use` lists profiles through `dialoguer::Select`.
- [ ] The active profile label includes `  active`.
- [ ] The active profile is highlighted by default when it exists; otherwise the first profile is highlighted.
- [ ] Empty profile config returns a clear error and does not write config.
- [ ] Non-TTY `cc-profile use` returns a clear error and does not write config.
- [ ] Canceling the selector returns successfully or cleanly without changing config.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- [ ] `cargo nextest run --workspace` passes.
- [ ] No placeholders remain.

## Project Standards

- Follow root `AGENTS.md`, which references `docs/specs`.
- Rust style, error handling, and dependency constraints: `docs/specs/rust.md`.
- Test placement and quality: `docs/specs/rust_testing.md`.
- Commit format: `docs/specs/git.md`.

## Implementation Plan

**Hierarchy:** `Phase → Task → Step`

**Parallelism key:** `[P]` = parallel with siblings, `[S]` = sequential, `[S after Task X.Y]` = blocks on X.Y.

### Phase 1: CLI Dispatch and Selector

**Goal:** Deliver the user-visible command behavior behind `cc-profile use` with safe terminal handling.

#### Task 1.1: Optional `use` Profile Argument [S]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Update the clap subcommand model and dispatch logic so `cc-profile use` reaches application code with `None`, while `cc-profile use <profile>` still takes the unchanged direct activation path. This task does not implement the selector UI; it introduces the branch point and keeps the existing `use_profile` function as the direct path.

**Files:**
- Modify: `src/cli.rs`
- Test: `tests/integration/cli.rs`

**Code Preview:**

```rust
pub enum Command {
    Use {
        profile: Option<String>,
    },
}

Some(Command::Use { profile }) => match profile {
    Some(profile) => use_profile(&repository, &profile),
    None => use_profile_interactively(&repository),
},
```

**Steps (run by implementer):**

1. Write failing tests covering:
   - `cc-profile use profile-b` still succeeds and writes `active_profile = "profile-b"`.
   - `cc-profile use` no longer fails with clap's missing required `<PROFILE>` error; until Task 1.2, assert it reaches a temporary application-level error.
2. Run tests — expect FAIL because `profile` is still required.
3. Implement the minimum parser and dispatch changes, with a temporary `use_profile_interactively` stub returning a clear not-yet-implemented application error.
4. Run tests — expect PASS for this task's scoped behavior.
5. Commit: `git commit -m "feat(cli): accept omitted use profile"`.

**Validation (tester):**
- `cargo nextest run --workspace cli::use_command_sets_active_profile` passes.
- Focused missing-argument integration test proves the error no longer comes from clap's required `<PROFILE>` validation.
- `cargo fmt --check` passes.
- No changes to unrelated subcommand behavior.

#### Task 1.2: Interactive Profile Selector [S after Task 1.1]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Replace the temporary stub with a dedicated selector for the missing-profile path. The selector loads config, refuses empty profile sets and non-TTY stdin with clear errors, builds labels that preserve raw profile names separately from display labels, uses the active profile as the default when present, and calls `use_profile` only after a confirmed selection. It does not expose this helper as public API.

**Files:**
- Modify: `src/cli.rs`
- Test: `src/cli.rs`

**Code Preview:**

```rust
struct ProfileSelectionEntry {
    name: String,
    label: String,
}

fn profile_selection_entries(config: &Config) -> Vec<ProfileSelectionEntry> {
    // Preserve raw `name`; labels may include "  active".
}
```

**Steps (run by implementer):**

1. Write failing tests covering:
   - Entries include every configured profile in deterministic order.
   - Active profile label is rendered as `<name>  active` without mutating the raw selected name.
   - Default index points to the active profile when present.
   - Default index is `0` when no active profile exists or the active profile is stale.
   - Empty profile config returns a clear error before opening dialoguer.
2. Run tests — expect FAIL because selector helpers do not exist.
3. Implement helper functions, stdin TTY guard, `Select::new().with_prompt("Select a profile").items(...).default(...).interact_opt()?`, and cancel handling that performs no config write.
4. Run tests — expect PASS.
5. Commit: `git commit -m "feat(cli): choose profile interactively"`.

**Validation (tester):**
- `cargo nextest run --workspace cli::` passes for all CLI integration tests.
- Unit tests in `src/cli.rs` pass.
- Manual TTY smoke check is documented for later execution: run `cargo run -- use`, press arrow/Enter, verify active profile changes.
- Non-TTY missing-profile path returns a clear error and exits non-zero without hanging.

**Phase 1 End Review:**
- `spec-reviewer` → Phase 1 goal met; tasks match plan; Success Criteria progress.
- `code-quality-reviewer` → code style, standards, no placeholders.
- Fix findings: `implementer` + `tester`, max 2 iterations, then move to next phase.
- **Gate:** pass to next phase after max 2 fix iterations.

### Phase 2: Regression Coverage and Verification

**Goal:** Lock the behavior with command-level regression coverage and repository verification.

#### Task 2.1: CLI Regression Tests and Verification [S after Task 1.2]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Strengthen integration tests around the final behavior that can be exercised without a TTY, and record the manual TTY check expected for the interactive selector. This task should avoid brittle pseudo-terminal test machinery unless the project already has it; helper unit tests cover selector construction, while integration tests cover parse/dispatch and non-TTY safety.

**Files:**
- Modify: `tests/integration/cli.rs`
- Optionally modify: `src/cli.rs` if test seams need small private-helper adjustments

**Code Preview:**

```rust
Command::cargo_bin("cc-profile")
    .expect("binary exists")
    .env("HOME", temp.path())
    .arg("use")
    .assert()
    .failure()
    .stderr(predicate::str::contains("requires an interactive terminal"));
```

**Steps (run by implementer):**

1. Write failing tests covering:
   - Missing profile in non-TTY exits with the selected user-facing error, not clap's `<PROFILE>` error.
   - Empty profile config exits with the selected user-facing error, not a panic or blank selector.
   - Direct profile selection still writes the config exactly as before.
2. Run tests — expect FAIL for any missing assertions or message mismatches.
3. Implement or adjust the minimum code needed to make tests stable and specific.
4. Run tests — expect PASS.
5. Commit: `git commit -m "test(cli): cover interactive use fallback"`.

**Validation (tester):**
- `cargo fmt --check` passes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- `cargo nextest run --workspace` passes.
- `cargo test --doc --workspace` passes.
- Manual TTY check is run if an interactive terminal is available; otherwise record that it was skipped because the test environment has no TTY.

**Phase 2 End Review:**
- `spec-reviewer` → Phase 2 goal met; all Success Criteria either verified by automated tests or explicitly marked as manual TTY behavior.
- `code-quality-reviewer` → code style, standards, no placeholders, no over-abstraction.
- Fix findings: `implementer` + `tester`, max 2 iterations, then finish.
- **Gate:** pass after max 2 fix iterations.
