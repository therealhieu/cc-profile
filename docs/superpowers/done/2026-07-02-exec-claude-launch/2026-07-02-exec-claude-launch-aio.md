# exec-claude-launch — All-in-One Plan

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

- **Issue 1:** Selecting `Start Claude` in the interactive menu returns to the `cc-profile` menu after Claude exits — *Evidence:* `src/interactive.rs:11-41` wraps the menu in `loop { ... }`, and the `"Start Claude"` arm only calls `launch::start_claude(&config)?` before the loop continues — *Solution:* Task 1.1 changes `launch::start_claude` to use Unix `exec`, so a successful launch replaces the current `cc-profile` process and cannot return to the menu.
- **Issue 2:** `cc-profile start` currently keeps an extra parent process around while Claude runs — *Evidence:* `src/services/launch.rs:75-91` uses `Command::status()`, which spawns a child process and waits for it; `src/cli.rs:269-272` calls the same launch path for the non-interactive `start` command — *Solution:* Task 1.1 makes the shared start path perform a true Unix handoff with `std::os::unix::process::CommandExt::exec`.
- **Issue 3:** The launch handoff should preserve existing config validation errors before attempting to execute Claude — *Evidence:* test `start_claude_propagates_build_command_spec_errors` in `src/services/launch.rs:229-233` checks that `start_claude(&Config::default())` fails with `No active profile is set` before launching — *Solution:* Task 1.1 keeps `build_command_spec(config)?` as the first operation inside the testable start helper, then calls the exec launcher only after a valid command spec exists.

## Goal

Make `cc-profile start` and interactive `Start Claude` replace the `cc-profile` process with Claude Code on Unix after validating the active profile config.

## Non-Goals

- Do not support Windows or non-Unix platforms for this change.
- Do not add a spawn-and-wait fallback for non-Unix platforms.
- Do not change profile selection, env var merging, Claude args, update checks, or config persistence.
- Do not modify the interactive menu structure; the shared launch behavior is the only path that changes.
- Do not change `run_command_spec`; it remains the spawn-and-wait helper.

## Current State

```text
interactive::run()
  └── loop
      ├── load config
      ├── render menu
      └── match selected option
          └── "Start Claude"
              └── launch::start_claude(&config)
                  ├── build_command_spec(config)
                  └── run_command_spec(spec)
                      └── Command::status()
                          ├── spawn child: claude
                          ├── wait for child exit
                          └── return Ok or error
              └── loop continues; menu renders again

cli::start_command()
  └── launch::start_claude(&config)
      └── same spawn-and-wait path
```

## Expected State

```text
interactive::run()
  └── loop
      ├── load config
      ├── render menu
      └── match selected option
          └── "Start Claude"
              └── launch::start_claude(&config)
                  ├── start_claude_with_launcher(config, exec_command_spec)
                  │   ├── build_command_spec(config)
                  │   └── exec_command_spec(spec)
                  └── CommandExt::exec()
                      └── current process image becomes: claude

cli::start_command()
  └── launch::start_claude(&config)
      └── same exec handoff path

On successful exec:
  cc-profile no longer exists as a separate process
  Claude owns the process, terminal, and exit status
```

The parent shell still waits on the same PID. After `exec`, that PID is running Claude instead of `cc-profile`, so Claude's final exit status is what the shell observes.

## Testing

- **Framework:** Rust unit tests, integration tests, and existing cargo checks.
- **TDD cycle:** failing test → run (FAIL) → implement → run (PASS) → commit.
- **Coverage target:** Keep repository coverage compatible with `cargo llvm-cov nextest --workspace --fail-under-lines 90` from `docs/specs/rust_testing.md`.
- **Test files:**
  - `src/services/launch.rs`
  - `tests/integration/launch.rs` remains a required regression signal; `start_launches_claude_with_profile_envs_and_configured_args` should still pass because `assert_cmd` waits on the original process PID, and after `exec` that PID runs the test shim, writes `CC_PROFILE_TEST_CLAUDE_OUTPUT`, and exits.

## Success Criteria

- [ ] `start_claude` still returns `No active profile is set` before launching when config has no active profile.
- [ ] Valid launch specs flow through `start_claude_with_launcher(config, exec_command_spec)` instead of `run_command_spec` / `Command::status`.
- [ ] `exec_command_spec` preserves the existing missing-program guidance when `Command::exec()` returns an error: `Could not find \`{program}\` on PATH. Please install Claude Code or ensure the \`{program}\` command is available.`
- [ ] Existing `run_command_spec` behavior remains unchanged for callers that need spawn-and-wait semantics.
- [ ] `git diff --stat -- src/interactive.rs` shows zero changed lines; no `break` is added to the menu loop.
- [ ] The crate checks successfully on the target Unix platform.
- [ ] `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo nextest run --workspace`, `cargo test --doc --workspace`, `cargo llvm-cov nextest --workspace --fail-under-lines 90`, and `cargo deny check` pass, or `./scripts/ci.sh` passes if it runs these commands.
- [ ] `rg -n "TODO|FIXME|XXX|TBD|fill in later" src/services/launch.rs` returns no placeholders introduced by this change.

## Project Standards

- Global instructions: `/Users/hieunguyen/.config/opencode/AGENTS.md` — concise, pragmatic, verify claims.
- Project agent entrypoint: `AGENTS.md` → `docs/specs`.
- Development workflow: `docs/specs/development.md`.
- Rust standards: `docs/specs/rust.md`.
- Rust testing standards: `docs/specs/rust_testing.md`.
- Git standards: `docs/specs/git.md`.
- API references verified during planning: `/rust-lang/rust` docs for `std::os::unix::process::CommandExt::exec` and `std::process::Command::status`; POSIX `exec` behavior confirms successful exec replaces the current process image and does not return.

## Implementation Plan

**Hierarchy:** `Phase → Task → Step`

**Parallelism key:** `[P]` = parallel with siblings, `[S]` = sequential, `[S after Task X.Y]` = blocks on X.Y.

### Phase 1: Unix exec launch handoff

**Goal:** Replace the current spawn-and-wait start path with a Unix process handoff while preserving pre-launch validation errors.

#### Task 1.1: Convert `start_claude` to exec handoff [S]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Update the launch service so `start_claude` builds the existing `CommandSpec`, then invokes a Unix-only exec helper that replaces the current process with `claude`. Keep `run_command_spec` unchanged. Add one private module helper, `start_claude_with_launcher`, that accepts the launcher function so unit tests can prove valid configs reach the handoff without replacing the test process. Do not change command-spec construction, env var ordering, args ordering, CLI routing, or interactive menu options. Because this feature is Unix-only, `exec_command_spec` and the `CommandExt` import are `#[cfg(unix)]`; no non-Unix fallback is added.

**Files:**
- Modify: `src/services/launch.rs`

**Code Preview:**

```rust
pub fn start_claude(config: &Config) -> Result<()> {
    start_claude_with_launcher(config, exec_command_spec)
}

fn start_claude_with_launcher<F>(config: &Config, launch: F) -> Result<()>
where
    F: FnOnce(&CommandSpec) -> Result<()>,
{
    let spec = build_command_spec(config)?;
    launch(&spec)
}
```

`exec_command_spec` uses `CommandExt::exec()` and preserves the full existing missing-program message from `run_command_spec` when `exec` returns an error.

**Test capture preview:**

```rust
let captured = std::cell::RefCell::new(None);
start_claude_with_launcher(&config, |spec| {
    *captured.borrow_mut() = Some(spec.clone());
    Ok(())
})?;
assert_eq!(captured.into_inner(), Some(expected_spec));
```

**Steps (run by implementer):**

1. Write failing tests in `src/services/launch.rs` covering:
   - Leave `start_claude_propagates_build_command_spec_errors` intact so it still fails before launch for `Config::default()`.
   - Add a new `start_claude_with_launcher_invokes_launcher_with_built_spec` test that passes a valid config to `start_claude_with_launcher`, captures the `CommandSpec` with the Test capture preview pattern, and returns `Ok(())` instead of calling `exec`.
   - Add an `exec_command_spec_returns_context_when_exec_fails` test that uses a direct `CommandSpec` with a guaranteed-missing program name and asserts the full missing-program guidance if `Command::exec()` returns before replacing the process.
   - Existing or added `run_command_spec` coverage still describes spawn-and-wait behavior and is not rewritten as exec behavior.
2. Run `cargo nextest run --workspace start_claude` and expect FAIL because the new `start_claude_with_launcher_*` test does not have an implementation yet.
3. Implement the minimum code to pass:
   - Add `#[cfg(unix)] use std::os::unix::process::CommandExt;`.
   - Introduce private module helper `start_claude_with_launcher<F>(config, launch)` exactly as shown in the Code Preview.
   - Change `start_claude` to call `start_claude_with_launcher(config, exec_command_spec)`.
   - Add `#[cfg(unix)] fn exec_command_spec(&CommandSpec) -> Result<()>` that calls `Command::new(&spec.program).args(&spec.args).envs(&spec.envs).exec()`.
   - Convert the returned `io::Error` into `anyhow::Error` with the full existing message: `Could not find \`{program}\` on PATH. Please install Claude Code or ensure the \`{program}\` command is available.`
   - Keep `run_command_spec` unchanged.
4. Run these focused validations and expect PASS:
   - `cargo nextest run --workspace start_claude`
   - `cargo nextest run --workspace exec_command_spec`
   - `cargo nextest run --workspace start_launches_claude_with_profile_envs_and_configured_args`
5. Commit: `git commit -m "fix(launch): exec claude on start"`.

**Validation (tester):**
- `uname -s` confirms the tester is on a Unix platform such as Darwin or Linux.
- `cargo nextest run --workspace start_claude` passes.
- `cargo nextest run --workspace exec_command_spec` passes.
- `cargo nextest run --workspace start_launches_claude_with_profile_envs_and_configured_args` passes.
- `cargo nextest run --workspace launch` passes.
- `cargo fmt --check` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- `git diff --stat -- src/interactive.rs` prints no changed lines.
- The diff shows `run_command_spec` still using `Command::status()`.
- The valid-config unit test does not call the real `claude` binary and cannot exec the test process.
- `rg -n "TODO|FIXME|XXX|TBD|fill in later" src/services/launch.rs` returns no matches introduced by this change.

**Phase 1 End Review:**
- `spec-reviewer` → Phase 1 goal met; implementation matches Unix-only exec handoff; non-goals remain out of scope.
- `code-quality-reviewer` → launch code remains simple, test seam is narrow and private, error context is clear, no placeholders.
- Fix findings: `implementer` + `tester`, max 2 iterations, then move to completion.
- **Gate:** pass to done after max 2 fix iterations.
