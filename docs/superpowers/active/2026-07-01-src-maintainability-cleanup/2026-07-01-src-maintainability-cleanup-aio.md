# src-maintainability-cleanup — All-in-One Plan

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
  - [Phase 1: Quick wins](#phase-1-quick-wins)
  - [Phase 2: Replace hand-rolled date/time with Unix timestamps](#phase-2-replace-hand-rolled-datetime-with-unix-timestamps)
  - [Phase 3: Config load-mutate-save helper](#phase-3-config-load-mutate-save-helper)
  - [Phase 4: Index-based menu selection](#phase-4-index-based-menu-selection)

## Problem

A `src` review of `cc-profile` found four maintainability liabilities. None are runtime defects today, but each raises the cost and risk of future change.

**Issue 1 — ~130 lines of hand-rolled RFC3339 + Gregorian calendar math** in `update_check_cache.rs`, including a `.expect()` panic path, used only for a 24-hour staleness check:

```rust
// update_check_cache.rs:204-206 — panic path in a passive background check
let year: i32 = y.try_into().map_err(|_| ())
    .expect("civil year out of i32 range");
// plus days_from_civil / civil_from_days_i64 / format_utc_from_unix / parse_rfc3339_utc
```

**Issue 2 — repeated load→mutate→save boilerplate with redundant double reads** across ~10 call sites:

```rust
// interactive.rs:137 render load, then :162 a SECOND load before mutating
let config = repository.load()?;          // top of loop, for the screen
// ... user picks "Set active" ...
let mut config = repository.load()?;      // reloaded again
profiles::set_active_profile(&mut config, name)?;
repository.save(&config)?;                // same 3-line dance in cli.rs + interactive.rs
```

**Issue 3 — interactive menus reconstruct a profile name from its display label**, coupling presentation to data:

```rust
// interactive.rs:130 — a profile literally named "foo  active" is mangled
let profile_name = selected_option.trim_end_matches("  active").to_string();
```

**Issue 4 — the four model→env-var mappings in `launch.rs` are hand-enumerated inline**, so adding a model means editing scattered inserts:

```rust
// launch.rs:45-60 — key names and fields spread across four separate calls
envs.insert("ANTHROPIC_DEFAULT_FABLE_MODEL".to_string(), profile.fable.clone());
envs.insert("ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(), profile.opus.clone());
envs.insert("ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(), profile.sonnet.clone());
envs.insert("ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(), profile.haiku.clone());
```

## Goal

Remove the highest-risk maintainability liabilities in `src` without changing user-facing behavior.

## Non-Goals

- Collapsing `Profile`'s six typed fields into a map (loses compile-time exhaustiveness — deliberately kept).
- A full enum-per-menu dispatch rewrite in `interactive.rs` (only the fragile label round-trip is fixed).
- Any performance optimization (`.clone()`, full-config serialization) — the tool is I/O-bound on a tiny TOML file; these are already appropriate.
- Adding a new date/time crate dependency (`jiff`/`time`) — the timestamp approach removes the need entirely.
- Changing the TOML config schema for profiles, envs, or args.

## Current State

```
update_check_cache.rs (staleness check)
  UpdateCheckCache { last_checked_at: String /* RFC3339 */, latest_seen }
        │
        ├── format_rfc3339_utc(SystemTime) ──> format_utc_from_unix ──> civil_from_days_i64  [.expect panic]
        └── parse_rfc3339_utc(&str) ─────────> days_from_civil (weak: accepts 2026-02-31)
                                        ~130 lines of calendar arithmetic

interactive.rs / cli.rs (mutations)
  let mut config = repository.load()?;        // often a SECOND load after a render load
  service::mutate(&mut config, ...)?;
  repository.save(&config)?;                   // repeated ~10x, no helper

interactive.rs (profile selection)
  option label = "profile-a  active"
  profile_name = label.trim_end_matches("  active")   // breaks on name "foo  active"

launch.rs (env build)
  envs.insert("ANTHROPIC_DEFAULT_FABLE_MODEL", profile.fable.clone());   // 4 inline inserts
  envs.insert("ANTHROPIC_DEFAULT_OPUS_MODEL",  profile.opus.clone());
  ...
```

## Expected State

```
update_check_cache.rs (staleness check)
  UpdateCheckCache { last_checked_unix: u64, latest_seen }
        │
        └── now.duration_since(UNIX_EPOCH) compared to interval    // no calendar math, no panic
  read_cache: malformed/old-format file ──> Ok(None)               // self-heals legacy string caches

repository.rs (mutations)
  pub fn update(&self, mutate: impl FnOnce(&mut Config) -> Result<()>) -> Result<Config>
  // callers: repository.update(|c| profiles::set_active_profile(c, name))?;   single load+save

interactive.rs (profile selection)
  select index ──> &profile_names[index]      // no label parsing; name never reconstructed

launch.rs (env build)
  for (key, value) in [("ANTHROPIC_DEFAULT_FABLE_MODEL", &profile.fable), ...] {
      envs.insert(key.to_string(), value.clone());
  }                                            // keys co-located, one edit point
```

## Testing

- **Framework:** Rust built-in `#[test]` (unit tests in-module) + integration tests in the single `integration` binary under `tests/integration/` (`main.rs` re-exports `cli`, `update`, `launch` modules; `assert_cmd`, `assert_fs`, `predicates`).
- **TDD cycle:** failing test → `cargo test` (FAIL) → implement → `cargo test` (PASS) → commit.
- **Full verification per task:** `cargo test` (unit + `integration` binary) **and** `cargo test --doc` (rustdoc examples on new/changed public items), then `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`. There is no `--test cli`/`--test update` target; the only integration target is `--test integration`.
- **Coverage target:** every changed public function retains or gains a unit test; behavior parity proven by test, not inspection.
- **Test files:**
  - `src/services/update_check_cache.rs` (in-module tests)
  - `src/services/update.rs` (in-module tests — construct `UpdateCheckCache`, reference stamp helper)
  - `src/config/repository.rs` (in-module tests — new `update` helper)
  - `src/services/launch.rs` (in-module tests — env mapping parity)
  - `src/interactive.rs` (in-module tests — selection helper)
  - `tests/integration/update.rs` (integration, in the single `integration` binary — must stay green; no cache-format coupling confirmed. Run with `cargo test --test integration`.)

## Success Criteria

- [ ] `update_check_cache.rs` contains no bespoke calendar functions (`days_from_civil`, `civil_from_days_i64`, `format_utc_from_unix`, `parse_rfc3339_utc`, `format_rfc3339_utc` removed or reduced to a single Unix-seconds helper).
- [ ] No `.expect(...)` remains on the passive-update-check code path.
- [ ] A legacy RFC3339-string cache file causes `read_cache` to return `Ok(None)` (self-heal), not an error.
- [ ] `ConfigRepository::update` exists and every load→mutate→save triple in `cli.rs` and `interactive.rs` uses it.
- [ ] No interactive code reconstructs a profile name via `trim_end_matches("  active")`; selection is index-based.
- [ ] The four `ANTHROPIC_DEFAULT_*_MODEL` inserts in `launch.rs` are driven by a single co-located table; `build_command_spec` output is byte-identical to before.
- [ ] Dead `let _ = yes;` in `cli.rs:139` removed.
- [ ] All tests pass (`cargo test --all-targets` and `cargo test --doc`).
- [ ] Lint and format pass (`cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`).
- [ ] `unix_secs` and other new internal helpers are `pub(crate)`, not `pub` (public API not widened).
- [ ] No placeholders remain.

## Project Standards

Cite, do not restate:

- `docs/specs/rust.md` — Rust standards (correctness-first, encode invariants in types, validate at boundaries, delete unused code, no speculative abstraction, narrow public APIs, rustdoc on public items).
- `docs/specs/rust_testing.md` — fast deterministic unit/integration tests for success, error, and edge paths.
- `docs/specs/development.md`, `docs/specs/git.md` — workflow and commit conventions.
- `AGENTS.md` → `@docs/specs`. Formatting owned by `cargo fmt`; lint via `cargo clippy`.

Key rule applied here: consolidate demonstrated repetition, but keep `Profile` field-typed (correctness/exhaustiveness over map-driven brevity).

## Implementation Plan

**Hierarchy:** `Phase → Task → Step`

**Parallelism key:** `[P]` = parallel with siblings, `[S]` = sequential, `[S after Task X.Y]` = blocks on X.Y.

### Phase 1: Quick wins

**Goal:** Land the two smallest, lowest-risk cleanups (dead code + launch env table) to warm up the test loop.

#### Task 1.1: Remove dead `let _ = yes;` [P]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** In `cli.rs`, `update_command` binds `let _ = yes;` then immediately uses `yes` in the struct literal below it. The discard line is dead. Remove it only. Do not change dispatch behavior or the `UpdateOptions` construction. No public API change.

**Files:**
- Edit: `src/cli.rs`

**Code Preview:**

```rust
// crucial: `yes` is already consumed by skip_confirm below — the discard is dead
fn update_command(check: bool, yes: bool) -> Result<()> {
    update::run_update(update::UpdateOptions {
        check_only: check,
        skip_confirm: yes,
    })
}
```

**Steps (run by implementer):**

1. Confirm existing `tests/integration/update.rs` and `tests/integration/cli.rs` tests cover the update dispatch path (`--check`, `--yes`); if a direct assertion on `update_command` behavior is absent, no new test is required since behavior is unchanged and covered by integration — note this in the commit body.
2. Run `cargo test` — expect PASS (baseline).
3. Remove the `let _ = yes;` line.
4. Run `cargo test` + `cargo clippy --all-targets -- -D warnings` — expect PASS.
5. Commit: `git commit -m "refactor(cli): drop dead let _ = yes binding"`

**Validation (tester):**
- Full test suite passes.
- `cargo clippy --all-targets -- -D warnings` clean (no unused-variable warning reintroduced).
- No regression in update command dispatch.
- `cargo fmt --check` clean.

#### Task 1.2: Table-drive model env-var inserts in `launch.rs` [P]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Replace the four separate `envs.insert("ANTHROPIC_DEFAULT_*_MODEL", profile.<field>.clone())` calls in `build_command_spec_with_program` with a single co-located `[(&str, &String)]` array iterated once. The two non-model inserts (`ANTHROPIC_BASE_URL`, `ANTHROPIC_API_KEY`) may stay explicit or join the table — implementer's choice, but output must be identical. Profile-after-global ordering and all key names must be preserved exactly. No signature change.

**Files:**
- Edit: `src/services/launch.rs`

**Code Preview:**

```rust
// crucial: keys co-located with fields; iteration order + names must match the old inserts
let model_envs = [
    ("ANTHROPIC_DEFAULT_FABLE_MODEL", &profile.fable),
    ("ANTHROPIC_DEFAULT_OPUS_MODEL", &profile.opus),
    ("ANTHROPIC_DEFAULT_SONNET_MODEL", &profile.sonnet),
    ("ANTHROPIC_DEFAULT_HAIKU_MODEL", &profile.haiku),
];
for (key, value) in model_envs {
    envs.insert(key.to_string(), value.clone());
}
```

**Steps (run by implementer):**

1. The existing test `build_command_spec_uses_active_profile_envs_after_global_envs` already asserts all four model keys + values and the profile-wins-over-global ordering. Confirm it covers every key; if any assertion is missing, add it first (FAIL).
2. Run `cargo test launch` — expect PASS (baseline) or FAIL (if a new assertion was added).
3. Apply the table refactor.
4. Run `cargo test launch` — expect PASS.
5. Commit: `git commit -m "refactor(launch): table-drive model env inserts"`

**Validation (tester):**
- `build_command_spec` produces byte-identical `CommandSpec.envs` for a representative profile (all six `ANTHROPIC_*` keys present with correct values).
- Global-env-then-profile override precedence unchanged.
- Full suite + clippy + fmt pass.

**Phase 1 End Review:**
- `spec-reviewer` → Phase 1 goal met; only dead code + env table changed; Success Criteria items for #6 and #4 satisfied; no scope drift.
- `code-quality-reviewer` → rustfmt/clippy clean; no placeholders; table is genuinely clearer than the inline inserts (per `rust.md` simplicity rule).
- Fix findings: `implementer` + `tester`, max 2 iterations, then move to next phase.
- **Gate:** pass to next phase after max 2 fix iterations.

### Phase 2: Replace hand-rolled date/time with Unix timestamps

**Goal:** Delete the ~130-line bespoke calendar module and its panic path by storing the last-check time as Unix seconds, with legacy caches self-healing.

#### Task 2.1: Migrate cache to Unix seconds and update all callers atomically [S]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** One atomic change across `update_check_cache.rs` **and** `update.rs` — the two modules must move together in a single commit, because deleting `format_rfc3339_utc` while `update.rs` still imports it (`update.rs:14`) and calls it (`update.rs:187`, test `update.rs:842`) leaves the crate uncompilable. In `update_check_cache.rs`: change `UpdateCheckCache.last_checked_at: String` to `last_checked_unix: u64`; rewrite `is_eligible_for_passive_check` to compare elapsed seconds against `min_interval`; delete `format_rfc3339_utc`, `format_utc_from_unix`, `parse_rfc3339_utc`, `days_from_civil`, and `civil_from_days_i64` (the `.expect` panic path); add a `pub(crate) unix_secs(SystemTime) -> Result<u64>` helper (error, not panic, if before epoch); make `read_cache` return `Ok(None)` on any deserialize failure (legacy RFC3339 string → treated as "no cache" → eligible → overwritten on next successful check), documented via rustdoc. In `update.rs`: replace both `write_cache` construction sites (`UpdateAvailable` + `Current` branches) to use `unix_secs(now)?` into `last_checked_unix`, and swap the import `format_rfc3339_utc` → `unix_secs`. Keep `latest_seen` and `skip_serializing_if`. No behavior change to notice printing, env-disable, or write-on-success semantics. `unix_secs` is `pub(crate)` (both callers are in-crate; do not widen the public API).

**Files:**
- Edit: `src/services/update_check_cache.rs`
- Edit: `src/services/update.rs`

**Code Preview:**

```rust
// crucial: staleness is pure elapsed-seconds math — no calendar, no panic
pub struct UpdateCheckCache {
    pub last_checked_unix: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_seen: Option<String>,
}

/// Whole seconds since the Unix epoch.
///
/// # Errors
/// Returns an error when `time` is before `UNIX_EPOCH`.
pub(crate) fn unix_secs(time: SystemTime) -> Result<u64> {
    Ok(time
        .duration_since(UNIX_EPOCH)
        .context("system time before Unix epoch")?
        .as_secs())
}

/// Missing OR unparseable (e.g. legacy RFC3339 string) cache is treated as absent,
/// so the next successful check rewrites it in the current format.
pub fn read_cache(path: &Path) -> Result<Option<UpdateCheckCache>> { /* Ok(None) on parse error */ }
```

**Steps (run by implementer):**

1. Rewrite in-module tests first — in `update_check_cache.rs`: eligibility (no cache / recent / stale via `UNIX_EPOCH + Duration`), `u64` round-trip read/write, and a legacy `last_checked_at = "..."` TOML string yielding `Ok(None)` from `read_cache`; in `update.rs`: the three passive-check tests (`passive_update_check_skips_lookup_when_cache_recent`, `passive_update_check_prints_notice_and_writes_cache`, and the env-disabled one) rebuilt around `last_checked_unix` / `unix_secs(now)`. Run — expect FAIL/compile error.
2. Run `cargo test --lib` — expect FAIL (both modules).
3. Implement both modules together: new struct field, `pub(crate) unix_secs`, rewritten eligibility, tolerant `read_cache`, deletion of the five calendar/RFC3339 functions, and both `update.rs` call sites + import swapped.
4. Run `cargo test` (full, includes the `integration` binary) + `cargo clippy --all-targets -- -D warnings` — expect PASS.
5. Commit: `git commit -m "refactor(update-check): store last check as unix seconds, drop calendar math"`

**Validation (tester):**
- Crate compiles (no dangling `format_rfc3339_utc` import/call); no `.expect` on the passive-check path (grep clean).
- Legacy string-format cache file → `read_cache` returns `Ok(None)`, no error surfaced.
- Eligibility parity: recent (<24h) skips, stale (≥24h) and missing are eligible.
- Passive check still: skips when env-disabled, skips when cache recent, prints notice + writes cache when update available, writes cache (no notice) when current.
- `cargo test --test integration` green (covers `tests/integration/update.rs`).
- Full suite + clippy + fmt pass.

**Phase 2 End Review:**
- `spec-reviewer` → calendar functions gone; no `.expect` on the path; legacy-cache self-heal proven by test; Success Criteria #1, #2 (panic), #3 satisfied; passive-check behavior unchanged.
- `code-quality-reviewer` → net line reduction; `unix_secs` is `pub(crate)` not `pub`; `read_cache` tolerance documented with rustdoc; clippy/fmt clean; no dead imports.
- Fix findings: `implementer` + `tester`, max 2 iterations, then move to next phase.
- **Gate:** pass to next phase after max 2 fix iterations.

### Phase 3: Config load-mutate-save helper

**Goal:** Introduce `ConfigRepository::update` and route every load→mutate→save triple (and eliminate redundant double reads) through it.

#### Task 3.1: Add `ConfigRepository::update` [S]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Add a public method that loads the config, applies a fallible mutator closure, saves, and returns the updated `Config`. If the mutator returns `Err`, propagate without saving (no partial write). Add rustdoc per `rust.md`. Do not yet change callers — this task only adds and unit-tests the helper.

**Files:**
- Edit: `src/config/repository.rs`

**Code Preview:**

```rust
// crucial: mutator error must NOT save; returns the saved config for callers that print it
pub fn update(&self, mutate: impl FnOnce(&mut Config) -> Result<()>) -> Result<Config> {
    let mut config = self.load()?;
    mutate(&mut config)?;
    self.save(&config)?;
    Ok(config)
}
```

**Steps (run by implementer):**

1. Write in-module tests: (a) successful mutate persists and round-trips; (b) mutator returning `Err` leaves the on-disk file unchanged (or absent if it never existed) and propagates the error. Run — expect FAIL.
2. Run `cargo test repository` — expect FAIL.
3. Implement `update`.
4. Run `cargo test repository` — expect PASS.
5. Commit: `git commit -m "feat(config): add ConfigRepository::update helper"`

**Validation (tester):**
- Error path performs no write (assert file bytes unchanged / still absent).
- Success path persists and reloads equal.
- Full suite + clippy + fmt pass.

#### Task 3.2: Route `cli.rs` mutations through `update` [S after Task 3.1]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Convert the load→mutate→save triples in `cli.rs` (`use_profile`, `create_profile_command`, `edit_profile_command`, `delete_profile_command`) to `repository.update(...)`. Preserve all `println!` output and the `was_active`/`set_active` messaging (compute needed pre-state inside or around the closure as required). Behavior and stdout must be identical. `edit_profile_command`'s rename-then-edit sequence must remain correct (may use one `update` closure that performs rename + field edits, or sequential `update` calls — implementer's choice, but the final saved state must match today's).

**Files:**
- Edit: `src/cli.rs`

**Steps (run by implementer):**

1. Confirm integration coverage in `tests/integration/cli.rs` for use/new/edit/delete; add assertions for any output line not currently pinned (rename path especially). Run — expect FAIL if new assertions added, else PASS baseline.
2. Run `cargo test --test integration` — establish baseline.
3. Refactor the four commands to `update`.
4. Run `cargo test` (full) — expect PASS.
5. Commit: `git commit -m "refactor(cli): route mutations through ConfigRepository::update"`

**Validation (tester):**
- `tests/integration/cli.rs` output for use/new/edit(+rename)/delete unchanged.
- Error cases (missing profile, duplicate name) still fail with no partial write.
- Full suite + clippy + fmt pass.

#### Task 3.3: Route `interactive.rs` mutations through `update` and drop double reads [S after Task 3.1]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** In `interactive.rs`, replace the mutate arms that do a second `repository.load()?` after the render-time load (`profile_detail_menu` "Set active"/"Delete", `args_menu` toggle, `envs_menu` add, `edit_env_flow`, `delete_env_flow`, `new_profile_flow`, `edit_profile_flow`) with `repository.update(...)`. The top-of-loop render load stays (it feeds the screen); only the redundant re-load inside the action arm is removed. Preserve all prompts, confirmations, and printed messages. Menu control flow (loops, `return Ok(())` on delete) unchanged.

**Files:**
- Edit: `src/interactive.rs`

**Steps (run by implementer):**

1. The interactive flows are prompt-driven; rely on existing in-module unit tests (`render_main_screen`, `profile_options`, `apply_profile_field_update`, `env_options`) plus manual reasoning. Add a unit test only where a pure helper changes. Run baseline — expect PASS.
2. Run `cargo test interactive` — baseline.
3. Refactor each mutate arm to `update`; delete the redundant second `load()`.
4. Run `cargo test` (full) + `cargo clippy` — expect PASS.
5. Commit: `git commit -m "refactor(interactive): use update helper, drop redundant config reads"`

**Validation (tester):**
- No `repository.load()?` immediately followed by `service::...(&mut config)` + `save` remains (grep for the double-load pattern).
- All interactive unit tests pass; behavior of pure render/option helpers unchanged.
- Full suite + clippy + fmt pass. State explicitly that prompt-driven flows are not auto-testable and were verified by code inspection.

**Phase 3 End Review:**
- `spec-reviewer` → `update` helper exists and is used at every triple in `cli.rs` + `interactive.rs`; no redundant double reads; Success Criteria #4 satisfied; no behavior/stdout drift.
- `code-quality-reviewer` → closures read cleanly, no partial-write on error, rustdoc present, clippy/fmt clean.
- Fix findings: `implementer` + `tester`, max 2 iterations, then move to next phase.
- **Gate:** pass to next phase after max 2 fix iterations.

### Phase 4: Index-based menu selection

**Goal:** Remove the fragile `trim_end_matches("  active")` name reconstruction so profile identity never round-trips through a display label.

#### Task 4.1: Select profiles by index, not by label parsing [S]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** In `interactive.rs::profile_menu`, build the menu entries as a single `Vec<(String, String)>` of `(name, label)` pairs (from `config.profiles.keys()`), so each display label is bound to its raw name in one value — no separate vectors to keep aligned. Feed the labels to `Select` and, on selection, resolve the chosen profile name from the same pair by index instead of `selected_option.trim_end_matches("  active")`. The "Back" entry (appended by the caller) keeps its index-based guard: a selection `>= entries.len()` is Back. Display labels (`"name  active"`) are unchanged on screen. Add the pair-building helper as a pure function so the mapping is unit-testable, and test that a profile literally named `foo  active` resolves to its exact name.

**Files:**
- Edit: `src/interactive.rs`

**Code Preview:**

```rust
// crucial: name and label live in ONE tuple, so there is no cross-vector alignment invariant to break
/// Returns `(name, display-label)` pairs for the profile menu, in `config.profiles` key order.
/// The active profile's label is suffixed with `"  active"`; the raw name is never reconstructed
/// from the label. Callers append a "Back" entry; a selection index `>= entries.len()` is Back.
fn profile_menu_entries(config: &Config) -> Vec<(String, String)> {
    config.profiles.keys().map(|name| {
        let label = if config.active_profile.as_deref() == Some(name.as_str()) {
            format!("{name}  active")
        } else {
            name.clone()
        };
        (name.clone(), label)
    }).collect()
}
```

**Steps (run by implementer):**

1. Write a unit test for `profile_menu_entries`: active profile gets the `"  active"` label; a profile literally named `foo  active` yields a pair whose `.0` is the exact name `foo  active` (never reconstructed from the label); entry order matches `config.profiles` key order. Run — expect FAIL.
2. Run `cargo test interactive` — expect FAIL.
3. Implement the helper and switch `profile_menu` to index-based resolution; delete the `trim_end_matches("  active")` line.
4. Run `cargo test interactive` — expect PASS.
5. Commit: `git commit -m "refactor(interactive): resolve profile selection by index"`

**Validation (tester):**
- Grep confirms no `trim_end_matches("  active")` remains.
- A profile named `foo  active` selects the correct profile (unit test).
- Active-profile labeling on screen unchanged; "Back" still exits.
- Full suite + clippy + fmt pass.

**Phase 4 End Review:**
- `spec-reviewer` → Success Criteria #5 satisfied; label parsing gone; no scope creep into a full enum-dispatch rewrite (out of scope per Non-Goals); all Success Criteria now checked.
- `code-quality-reviewer` → helper is cohesive and tested; name/label carried as one tuple (no parallel-vec invariant); clippy/fmt clean; no placeholders anywhere in the changeset.
- Fix findings: `implementer` + `tester`, max 2 iterations.
- **Gate:** final gate — when this review passes, the cleanup is complete.
