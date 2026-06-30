# config-toml-path — All-in-One Plan

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

The 2026-06-30 init design and current implementation store the config as a file at `~/.cc-profile`. The required storage layout is a directory plus TOML file: `~/.cc-profile/config.toml`. The current path prevents future colocated files under `~/.cc-profile/` and contradicts the requested config contract.

## Goal

Change cc-profile config storage, docs, tests, and manual checks from the home file `~/.cc-profile` to the config file `~/.cc-profile/config.toml`.

## Non-Goals

- Do not migrate an existing legacy `~/.cc-profile` file automatically.
- Do not change the TOML schema fields or config version.
- Do not add encrypted or keychain-backed credential storage.
- Do not change profile, env var, args, or Claude launch behavior beyond the config file path.

## Current State

```text
$HOME
  └── .cc-profile                  # TOML file
      ├── version = 1
      ├── active_profile
      ├── args
      ├── envs
      └── profiles

ConfigRepository::default_path()
  → dirs::home_dir()
  → home.join(".cc-profile")
```

## Expected State

```text
$HOME
  └── .cc-profile/                 # config directory, 0700 on Unix after save
      └── config.toml              # TOML file, 0600 on Unix
          ├── version = 1
          ├── active_profile
          ├── args
          ├── envs
          └── profiles

ConfigRepository::default_path()
  → dirs::home_dir()
  → home.join(".cc-profile").join("config.toml")
```

If `$HOME/.cc-profile` already exists as a file, saving must fail clearly instead of overwriting or silently migrating secrets.

## Testing

- **Framework:** Rust unit tests, integration tests with `assert_cmd`, `assert_fs`, `predicates`, and existing cargo checks.
- **TDD cycle:** failing test → run (FAIL) → implement → run (PASS) → commit.
- **Coverage target:** At least 90% total line coverage per `docs/specs/rust_testing.md`.
- **Test files:**
  - `src/config/repository.rs`
  - `tests/integration/cli.rs`
  - `tests/integration/launch.rs`
  - any other integration/manual docs discovered by `rg "\.cc-profile|config\.toml"`.

## Success Criteria

- [ ] `ConfigRepository::default_path()` returns `$HOME/.cc-profile/config.toml`.
- [ ] `ConfigRepository::save()` creates `$HOME/.cc-profile/` before writing `config.toml`.
- [ ] Unix `config.toml` is `0600` after save or explicit permission fix.
- [ ] Unix `.cc-profile/` is `0700` after save, including if it already existed with broader permissions.
- [ ] Existing `$HOME/.cc-profile` file conflict fails with a clear message and does not overwrite the file.
- [ ] `cc-profile show` prints `Config file: <home>/.cc-profile/config.toml`.
- [ ] Tests no longer create or assert `$HOME/.cc-profile` as the config file.
- [ ] Design, plan, goal, manual, and check artifacts describe `~/.cc-profile/config.toml`.
- [ ] `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo nextest run --workspace`, and `cargo test --doc --workspace` pass.

## Project Standards

- Global instructions: `/Users/hieunguyen/.config/opencode/AGENTS.md` — concise, pragmatic, verify claims.
- Development standards: `docs/specs/development.md`.
- Rust standards: `docs/specs/rust.md`.
- Rust testing standards: `docs/specs/rust_testing.md`.
- Git standards: `docs/specs/git.md`.
- Use the repository's configured git identity. Do not override author metadata.

## Implementation Plan

**Hierarchy:** `Phase → Task → Step`

**Parallelism key:** `[P]` = parallel with siblings, `[S]` = sequential, `[S after Task X.Y]` = blocks on X.Y.

### Phase 1: Repository Path Behavior

**Goal:** Make the code persist config at `~/.cc-profile/config.toml` and handle directory creation and conflicts safely.

#### Task 1.1: ConfigRepository default path and save semantics [S]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Update `ConfigRepository` so the default path is `$HOME/.cc-profile/config.toml`, `save()` creates or reuses the parent directory safely, Unix permissions are correct for both directory and file, and a legacy file at `$HOME/.cc-profile` causes a clear error. Do not change config schema, service logic, CLI command behavior, or launch env construction.

**Files:**
- Modify: `src/config/repository.rs`

**Code Preview:**

```rust
pub fn default_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".cc-profile").join("config.toml"))
}
```

**Steps (run by implementer):**

1. Write failing tests in `src/config/repository.rs` covering:
   - `default_path()` appends `.cc-profile/config.toml` under `HOME`.
   - `save_creates_config_directory_before_writing_config_toml`: `save()` creates `.cc-profile/config.toml` when `.cc-profile/` is missing.
   - `save_tightens_existing_config_directory_permissions`: on Unix, create `.cc-profile/` with `0755`, call `save()`, and assert the directory is now `0700`.
   - Existing `.cc-profile` file conflict returns an error and preserves file contents.
   - Existing tests that are not about the default path keep using a bare-file fixture like `temp.path().join("plain-config.toml")`; only new config-layout contract tests use `.cc-profile/config.toml`.
2. Run `cargo test config::repository --lib` and expect FAIL because the implementation still resolves `.cc-profile` as a file and does not create the parent directory.
3. Implement the minimum code to pass:
   - Change `default_path()` to `home.join(".cc-profile").join("config.toml")`.
   - Before `fs::write`, create the parent directory with clear error context.
   - Set the parent directory permission to `0700` on Unix after `create_dir_all`, whether newly created or pre-existing.
   - Keep config file permission behavior at `0600`.
   - Preserve and clarify the error when `.cc-profile` is already a file.
   - Keep these existing tests on bare-file fixtures such as `plain-config.toml`: `load_returns_default_config_when_file_is_missing`, `save_then_load_round_trips_toml_config`, `load_rejects_newer_config_version_without_overwriting_file`, and `existing_broad_permissions_are_reported_and_can_be_fixed`.
4. Run `cargo test config::repository --lib` and expect PASS.
5. Commit: `git commit -m "fix(config): store config in config.toml"`.

**Validation (tester):**
- `cargo test config::repository --lib` passes.
- Existing repository tests still pass on their bare-file fixtures such as `plain-config.toml` per Step 3.
- `rg -n 'temp\.path\(\)\.join\("\.cc-profile"\)' src/config/repository.rs` returns no config-file fixture references.
- `cargo fmt --check` passes.

**Phase 1 End Review:**
- `spec-reviewer` → Phase 1 goal met; storage path contract implemented; no schema or migration scope added.
- `code-quality-reviewer` → repository code remains simple, errors are clear, permissions are platform-gated, no placeholders.
- Fix findings: `implementer` + `tester`, max 2 iterations, then move to next phase.
- **Gate:** pass to next phase after max 2 fix iterations.

### Phase 2: Integration Tests and CLI Expectations

**Goal:** Update all automated tests and CLI-visible expectations to use `~/.cc-profile/config.toml`.

#### Task 2.1: Integration fixture paths [S after Task 1.1]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Change integration test fixtures and assertions from `$HOME/.cc-profile` as a file to `$HOME/.cc-profile/config.toml`. This includes CLI command tests and launch tests. Do not weaken assertions about API key display, active profile updates, env injection, or args behavior.

**Files:**
- Modify: `tests/integration/cli.rs`
- Modify: `tests/integration/launch.rs`
- Modify: other test files returned by `rg -n 'temp\.child\("\.cc-profile|join\("\.cc-profile' tests src`.

**Code Preview:**

```rust
temp.child(".cc-profile/config.toml").write_str("version = 1\n")?;
```

**Steps (run by implementer):**

1. Write or update failing integration expectations covering:
   - `use` updates `temp.child(".cc-profile/config.toml")`.
   - `new --active` creates `temp.child(".cc-profile/config.toml")`.
   - `show` stdout contains `.cc-profile/config.toml` in the `Config file:` line.
   - `start` reads `temp.child(".cc-profile/config.toml")` for active profile envs and args.
2. Run `cargo nextest run --workspace` and expect FAIL where fixtures still write/read `temp.child(".cc-profile")`.
3. Implement the minimum test/fixture changes:
   - Replace config-file fixture paths with `temp.child(".cc-profile/config.toml")`.
   - Add a small helper only if it removes repeated path literals without hiding assertions.
   - Update `Config file:` stdout predicates to include `config.toml` where asserted.
4. Run `cargo nextest run --workspace` and expect PASS.
5. Commit: `git commit -m "test(config): use config toml path"`.

**Validation (tester):**
- `cargo nextest run --workspace` passes.
- `rg -n 'temp\.child\("\.cc-profile"\)|join\("\.cc-profile"\)' tests src` returns no stale config-file fixture usage.
- `cc-profile show` integration output includes `config.toml`.

**Phase 2 End Review:**
- `spec-reviewer` → automated tests cover the new path and preserve behavior unrelated to path storage.
- `code-quality-reviewer` → fixtures are DRY enough, assertions remain meaningful, no broad test weakening.
- Fix findings: `implementer` + `tester`, max 2 iterations, then move to next phase.
- **Gate:** pass to next phase after max 2 fix iterations.

### Phase 3: Documentation and Artifact Alignment

**Goal:** Align the 2026-06-30 init artifacts with the new `~/.cc-profile/config.toml` contract.

#### Task 3.1: Design and plan artifact path updates [P with Task 3.2]

**Subagent:** `implementer` (TDD-style doc validation) → `tester` (validate)

**Scope:** Update source-of-truth design and implementation plan artifacts so they no longer say the config file is `~/.cc-profile`. Preserve wording that the config directory is fixed under the home directory and not a platform config directory. Do not rewrite unrelated design decisions. Coordinate with Task 3.2 because both tasks scan the same init artifact directory; grep failures from the sibling task's files are resolved at Phase 3 review.

**Files:**
- Modify: `docs/superpowers/active/2026-06-30-init/2026-06-30-init-design.md`
- Modify: `docs/superpowers/active/2026-06-30-init/2026-06-30-init-plan.md`
- Modify: `docs/superpowers/active/2026-06-30-init/2026-06-30-init-plan-1.md`
- Modify: `docs/superpowers/active/2026-06-30-init/2026-06-30-init-plan-3.md`
- Modify: `docs/superpowers/active/2026-06-30-init/2026-06-30-init-plan-4.md`
- Inspect and modify if grep finds stale file-path wording: `docs/superpowers/active/2026-06-30-init/2026-06-30-init-plan-2.md`

**Code Preview:**

```text
Config directory: ~/.cc-profile
Config file: ~/.cc-profile/config.toml
```

**Steps (run by implementer):**

1. Run `rg -n '~/.cc-profile|\.cc-profile' docs/superpowers/active/2026-06-30-init` and record stale locations in design and plan artifacts.
2. Treat each match that identifies `~/.cc-profile` as the config file as a failing doc check.
3. Update docs minimally:
   - Replace config-file references with `~/.cc-profile/config.toml`.
   - Use `~/.cc-profile/` only when referring to the directory.
   - Update diagrams so the directory contains `config.toml`.
   - If Step 1 finds stale file-path wording in `2026-06-30-init-plan-2.md`, apply the same file-vs-directory rewrites there.
4. Re-run `rg -n 'Config file: ~/.cc-profile(\b|$)|Default config path:|cat > "\$HOME/.cc-profile"' docs/superpowers/active/2026-06-30-init` and expect no stale matches in Task 3.1-owned files.
5. Commit: `git commit -m "docs(config): document config toml path"`.

**Validation (tester):**
- `rg -n 'Config file: ~/.cc-profile(\b|$)' docs/superpowers/active/2026-06-30-init` returns no matches after both Phase 3 tasks land.
- Design doc has explicit `Config directory: ~/.cc-profile` and `Config file: ~/.cc-profile/config.toml` wording.
- Plan file structure section names `src/config/repository.rs` as owning `~/.cc-profile/config.toml`.
- No unrelated design scope was changed.

#### Task 3.2: Manual and completion artifact path updates [P with Task 3.1]

**Subagent:** `implementer` (TDD-style doc validation) → `tester` (validate)

**Scope:** Update manual verification and completion/check artifacts so command examples create the config directory and write `config.toml`. Do not alter manual verification behavior except paths.

**Files:**
- Modify: `docs/superpowers/active/2026-06-30-init/2026-06-30-init-manual.md`
- Modify: `docs/superpowers/active/2026-06-30-init/2026-06-30-init-goal.md`
- Modify: `docs/superpowers/active/2026-06-30-init/2026-06-30-init-check.md`

**Code Preview:**

```bash
mkdir -p "$HOME/.cc-profile"
cat > "$HOME/.cc-profile/config.toml" <<'EOF'
```

**Steps (run by implementer):**

1. Run `rg -n '\$HOME/\.cc-profile|<tmp>/\.cc-profile|temporary-home>/.cc-profile|~/.cc-profile' docs/superpowers/active/2026-06-30-init/2026-06-30-init-{manual,goal,check}.md` and record stale file-path examples.
2. Treat examples that write directly to `$HOME/.cc-profile` or describe it as the config file as failing doc checks.
3. Update manual commands and expected outputs:
   - Insert `mkdir -p "$HOME/.cc-profile"` before heredocs.
   - Replace `cat > "$HOME/.cc-profile"` with `cat > "$HOME/.cc-profile/config.toml"`.
   - Replace invalid TOML examples to write `config.toml`.
   - Update expected `Config file:` lines to include `config.toml`.
4. Re-run `rg -n 'cat > "\$HOME/.cc-profile"|printf .* > "\$HOME/.cc-profile"|Config file: .*\.cc-profile$' docs/superpowers/active/2026-06-30-init/2026-06-30-init-{manual,goal,check}.md` and expect no matches.
5. Commit: `git commit -m "docs(config): update manual config path"`.

**Validation (tester):**
- Manual doc contains no `cat > "$HOME/.cc-profile"` or `printf 'not valid toml = [' > "$HOME/.cc-profile"` examples.
- Manual doc expected output says `.cc-profile/config.toml`.
- Goal/check files describe the delivered storage path as `~/.cc-profile/config.toml`.
- No manual step now writes secrets to a path that is a directory.

**Phase 3 End Review:**
- `spec-reviewer` → all artifacts consistently state directory vs file semantics and no acceptance criteria conflict remains.
- `code-quality-reviewer` → docs are precise, minimal, and free of stale contradictory wording.
- Fix findings: `implementer` + `tester`, max 2 iterations, then move to next phase.
- **Gate:** pass to next phase after max 2 fix iterations.

### Phase 4: Final Verification and Cleanup

**Goal:** Prove the path fix works across code, tests, docs, and repository search.

#### Task 4.1: Final verification sweep [S after Phase 3]

**Subagent:** `implementer` (verification fixes only if needed) → `tester` (validate)

**Scope:** Run the final checks, fix only issues caused by the config path change, and produce evidence. Do not add new features or refactor unrelated code.

**Files:**
- Modify only files already touched in Phases 1–3 if verification finds path-related misses.

**Code Preview:**

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --workspace
cargo test --doc --workspace
```

**Steps (run by implementer):**

1. Run final verification commands and stale-path grep:
   - `cargo fmt --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo nextest run --workspace`
   - `cargo test --doc --workspace`
   - `rg -n 'cat > "\$HOME/.cc-profile"|printf .* > "\$HOME/.cc-profile"|temp.child\(".cc-profile"\)|Config file: ~/.cc-profile(\b|$)' src tests docs/superpowers/active/2026-06-30-init`
2. Expect all commands to pass after Phases 1–3; any failure caused by this path change is a verification failure.
3. If a command fails due to this path change, make the smallest fix in the relevant code/test/doc file.
4. Re-run the failed command, then run the full final verification list again and expect PASS.
5. Commit any verification fixes: `git commit -m "fix(config): finish config path alignment"`. If no files changed, do not create an empty commit.

**Validation (tester):**
- `cargo fmt --check` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- `cargo nextest run --workspace` passes.
- `cargo test --doc --workspace` passes.
- Validation report includes the stale-path grep output: either zero lines, or each remaining line is listed and explicitly adjudicated as a valid directory reference rather than a config-file reference.

**Phase 4 End Review:**
- `spec-reviewer` → final implementation matches this AIO and user request.
- `code-quality-reviewer` → no quality regressions, no placeholders, no unrelated scope.
- Fix findings: `implementer` + `tester`, max 2 iterations, then finish.
- **Gate:** implementation complete after max 2 fix iterations.
