# sync-codex — All-in-One Plan

> Concise design + implementation plan in one file. TDD. Subagent-friendly.

## Table of Contents

- [Problem](#problem)
- [Goal](#goal)
- [Non-Goals](#non-goals)
- [Current State](#current-state)
- [Expected State](#expected-state)
- [Design Decisions](#design-decisions)
- [Testing](#testing)
- [Success Criteria](#success-criteria)
- [Project Standards](#project-standards)
- [Implementation Plan](#implementation-plan)

## Problem

cc-profile stores endpoint + API key per profile, but that configuration is locked to Claude Code (it only ever becomes `ANTHROPIC_*` env vars for the `claude` binary). A user who also runs Codex against the same endpoints has to re-enter every endpoint and key by hand in `~/.codex/config.toml`, and keep the two in sync manually.

- **Issue 1: cc-profile profiles cannot be reused by Codex.** — *Evidence:* The only consumer of a profile is `launch::build_command_spec` (`src/services/launch.rs:26`), which emits `ANTHROPIC_BASE_URL` / `ANTHROPIC_API_KEY` for `claude`. Nothing writes Codex's on-disk config. — *Solution:* Add `cc-profile sync codex`, which registers each profile as a Codex custom provider `[model_providers.<name>]` in `~/.codex/config.toml` (Task 2.1, 3.1).

- **Issue 2: A hand-maintained `~/.codex/config.toml` drifts from cc-profile and risks clobbering unrelated Codex settings.** — *Evidence:* A Codex config is hand-edited: it carries `model`, `approval_policy`, `[mcp_servers.*]`, other `[model_providers.*]`, and inline comments. A naive rewrite (parse-to-struct then `toml::to_string`) would drop comments, reorder keys, and delete every key cc-profile does not model. — *Solution:* Merge with `toml_edit` (format-preserving): overwrite only the `[model_providers.<name>]` sub-tables whose name matches a cc-profile profile; leave every other table, key, and comment byte-for-byte intact (Task 2.1).

- **Issue 3: Codex forbids some provider ids; a blind write produces an unloadable config.** — *Evidence:* The Codex config reference reserves the built-in ids `openai`, `ollama`, `lmstudio` — they cannot be reused as custom `[model_providers.<id>]`. A profile legitimately named `openai` in cc-profile would generate a config Codex rejects at startup. — *Solution:* Skip reserved-name profiles with a printed warning, sync the rest, and exit success (Task 1.2, 2.1).

## Goal

Add `cc-profile sync codex`, which merges every cc-profile profile into `~/.codex/config.toml` as a custom `[model_providers.<name>]` provider (endpoint → `base_url`, api_key → inline `Authorization: Bearer` header), preserving all other Codex config and skipping reserved provider ids.

## Non-Goals

- **No model sync.** Codex custom providers do not carry a model; the profile's `fable`/`opus`/`sonnet`/`haiku` fields are intentionally not written. Codex's top-level `model` key is left untouched.
- **No `wire_api` / extra provider keys.** "Use defaults" — emit only `name`, `base_url`, and the auth header; let Codex default everything else (`wire_api`, retries, etc.).
- **No deletion / reconciliation.** Providers in `~/.codex/config.toml` that do not correspond to a current cc-profile profile are never removed or altered.
- **No new cc-profile config fields.** The `Profile` struct (`src/config/model.rs:24`) is unchanged.
- **No project-local `.codex/` writes.** Codex ignores `model_providers` in project configs; sync targets user-level `~/.codex/config.toml` only.
- **Not added to the interactive menu in this plan's core** (Phase 3 adds it; see task marking).

## Current State

```
cc-profile profile <name>
  { endpoint, api_key, fable, opus, sonnet, haiku }
        │
        ▼
  launch::build_command_spec        ← ONLY consumer
        │
        ▼
  ANTHROPIC_* env vars ─► exec `claude`

~/.codex/config.toml  ── hand-maintained, cc-profile never touches it
```

## Expected State

```
cc-profile sync codex
        │
        ▼
  ConfigRepository.load()  ─► all profiles
        │
        ▼
  sync_codex::sync_providers(config, codex_path)
        │   for each profile <name>:
        │     reserved id (openai/ollama/lmstudio)? ─► warn + skip
        │     else overwrite [model_providers.<name>]:
        │         name         = "<name>"
        │         base_url     = "<endpoint>"
        │         http_headers = { Authorization = "Bearer <api_key>" }
        │   (toml_edit merge — all other tables/keys/comments preserved)
        ▼
  ~/.codex/config.toml   ← other providers, model, approval_policy, comments intact

~/.codex path resolution:
  CODEX_HOME (if set) ── else ── dirs::home_dir()/.codex   then /config.toml
```

## Design Decisions

Locked with the user during brainstorming:

| Decision | Choice | Rationale |
|---|---|---|
| Codex mapping unit | **Custom provider** (`[model_providers.<name>]`), not Codex profile overlay | User reframed: this is about providers, not profile overlays. Sidesteps the 4-models-to-1 gap entirely. |
| API key placement | **Inline** `http_headers = { Authorization = "Bearer <key>" }` | User: "inject directly in the config." Codex has no `api_key` field on a provider; `Authorization: Bearer` is the common OpenAI-wire convention. |
| Merge semantics | **Overwrite matching providers, preserve everything else** | User: "overwrite which existed in cc-profile, don't delete others." |
| `wire_api` & extras | **Omit — Codex defaults** | User: "use defaults." |
| Reserved ids | **Skip with warning**, sync continues | User: "use defaults" → non-fatal, don't abort the whole sync over one bad name. |
| Model | **Not synced** | Providers carry no model; nothing sensible to map from four Claude IDs. |

## Testing

- **Framework:** Rust built-in (`#[test]`), `cargo test`. Unit tests inline in `src/services/sync_codex.rs`; integration tests via `assert_cmd` + `assert_fs` + `predicates` as a module of the `tests/integration/main.rs` harness (add `mod sync;` to `main.rs` — a standalone `tests/sync.rs` would not compile into the aggregated harness).
- **TDD cycle:** failing test → `cargo test` (FAIL) → implement → `cargo test` (PASS) → commit.
- **Path override for tests:** `sync_codex` resolves the Codex config via a `CODEX_HOME` env override before falling back to `dirs::home_dir()/.codex`, mirroring the env-then-home idiom in `src/services/update_check_cache.rs:38-40` and `src/services/receipt.rs:31-33`. Integration tests set `.env("CODEX_HOME", temp.child(".codex").path())` (and `HOME` for the cc-profile side) so no real `~/.codex` is touched.
- **Coverage target:** ≥ 90% of new lines in `sync_codex.rs` (matches existing service-module density).
- **Test files:** `src/services/sync_codex.rs` (unit), `tests/integration/sync.rs` (e2e), `tests/integration/main.rs` (harness registration).

## Success Criteria

- [ ] `cc-profile sync codex` creates `~/.codex/config.toml` (and `~/.codex/`) when absent, containing one `[model_providers.<name>]` per non-reserved profile.
- [ ] Each synced provider has exactly `name`, `base_url`, and `http_headers.Authorization = "Bearer <api_key>"` — no `wire_api`, no model.
- [ ] Running against an existing `~/.codex/config.toml` preserves all unrelated tables, keys, and comments byte-for-byte (verified by a fixture containing `model`, a foreign `[model_providers.other]`, and a comment).
- [ ] A profile whose name is `openai`, `ollama`, or `lmstudio` is skipped with a warning on stderr; other profiles still sync and the command exits `0`.
- [ ] Re-running sync is idempotent for unchanged profiles (second run produces no content diff).
- [ ] The written file has `0o600` perms and its parent `0o700` (consistent with `ConfigRepository` security posture).
- [ ] All tests pass; `cargo fmt`, `cargo clippy` clean; no placeholders remain.

## Project Standards

- `./scripts/ci.sh` runs fmt, clippy, test, package, publish-dry-run — new code must pass all.
- Service modules are flat `pub mod <name>;` entries in `src/services/mod.rs` with no re-exports; callers use full paths (`crate::services::sync_codex::...`).
- `deny.toml` gates dependency licenses; `toml_edit` (dual MIT / Apache-2.0) is within the allow-list.
- File/dir permission posture: `0o600` / `0o700`, per `src/config/repository.rs:125-152` (reuse that pattern; do not re-implement).
- Profile names are already validated at creation (`validate_profile_name`, `src/config/validation.rs`: alphanumeric/dash/underscore only), so `[model_providers.<name>]` keys are TOML-safe by construction — sync does not re-validate.

## Implementation Plan

**Hierarchy:** `Phase → Task → Step`

**Parallelism key:** `[P]` = parallel with siblings, `[S]` = sequential, `[S after Task X.Y]` = blocks on X.Y.

### Phase 1: Codex config module (pure core)

**Goal:** A dependency-free, file-agnostic core that transforms a `Config` into merged Codex TOML and knows path resolution + reserved names — fully unit-testable without touching the real filesystem.

#### Task 1.1: Add `toml_edit` dependency [S]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Add `toml_edit` to `Cargo.toml` and confirm it is license-clean. This is the enabling step for a format-preserving merge; plain `toml` (already present) cannot preserve comments/ordering and would violate Success Criterion 3. Does NOT add any other dependency.

**Files:**
- Modify: `Cargo.toml` (add `toml_edit` under `[dependencies]`)

**Steps (run by implementer):**
1. Run `cargo tree -i toml_edit` first. **Note (verified 2026-07-15):** `toml 1.x` was refactored onto `toml_parser`/`toml_writer` and no longer pulls `toml_edit` transitively, so this returns no match. Add `toml_edit` as a fresh direct dependency at the latest 0.25.x (resolved: `toml_edit = "0.25.13"`); it shares `toml_datetime`/`winnow` with `toml 1.1.2`, so no duplicate sub-deps enter the tree.
2. Run `cargo build` to populate `Cargo.lock`; confirm `cargo tree -i toml_edit` shows exactly one version.
3. Run `cargo deny check licenses` (or `./scripts/ci.sh`'s deny step) — expect PASS (MIT/Apache-2.0 allowed).
4. Commit: `git commit -m "build: add toml_edit for format-preserving codex merge"`.

> **API note for Task 1.3:** `toml_edit` is on the 0.25 major line, not 0.22. The preview APIs (`DocumentMut` parse, `toml_edit::value()`, `InlineTable::new().insert()`, `Item::is_table_like()`, auto-vivifying mutable indexing) are all present and stable in 0.25.

**Validation (tester):**
- `cargo build` succeeds; `Cargo.lock` updated.
- `cargo tree -i toml_edit` shows a single version (no duplicate in the tree).
- `cargo deny check licenses` passes.
- No other dependency changed.

#### Task 1.2: Path resolution + reserved-name predicate [S after Task 1.1]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Create `src/services/sync_codex.rs` with the pure helpers: resolve the Codex `config.toml` path (`CODEX_HOME` override, else `~/.codex`), and a predicate for the three reserved provider ids. No file I/O yet. Register the module in `src/services/mod.rs`.

**Files:**
- Create: `src/services/sync_codex.rs`
- Modify: `src/services/mod.rs` (add `pub mod sync_codex;`, alphabetically after `self_replace`/before `update` per existing ordering)

**Code Preview:**

```rust
// crucial: CODEX_HOME points AT the .codex dir (Codex convention), not its parent.
const RESERVED_PROVIDER_IDS: [&str; 3] = ["openai", "ollama", "lmstudio"];

pub(crate) fn is_reserved_provider_id(name: &str) -> bool {
    RESERVED_PROVIDER_IDS.contains(&name)
}

/// `$CODEX_HOME/config.toml` if set, else `~/.codex/config.toml`.
pub fn codex_config_path() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("CODEX_HOME") {
        return Ok(PathBuf::from(dir).join("config.toml"));
    }
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".codex").join("config.toml"))
}
```

**Steps (run by implementer):**
1. Write failing tests: `is_reserved_provider_id` true for the three ids / false otherwise; `codex_config_path` honors `CODEX_HOME` and falls back to home. **Serialize the env-mutating tests behind a `static Mutex` guard** (mirror `src/services/update_test_env_lock.rs` / `lock_cc_profile_update_check_env`) — `cargo test` runs in parallel, so setting/unsetting the process-global `CODEX_HOME` in one test can race the home-fallback test and make it flaky. A scoped guard alone is insufficient; the lock is required.
2. Run tests — expect FAIL (module/functions absent).
3. Implement the two helpers + module registration.
4. Run tests — expect PASS.
5. Commit: `git commit -m "feat(sync): add codex path resolution and reserved-id check"`.

**Validation (tester):**
- New unit tests pass; full suite green, including under `cargo test` parallelism (run twice to catch a flaky race).
- Env-mutating tests share the serialization mutex; `CODEX_HOME` is not left set after tests.
- clippy + fmt clean.

#### Task 1.3: Format-preserving merge core [S after Task 1.2]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** The heart of the feature: a pure function that takes existing Codex TOML text (may be empty) + a `&Config` and returns merged TOML text. Overwrites `[model_providers.<name>]` for each non-reserved profile with `name`/`base_url`/`http_headers.Authorization`; preserves every other table, key, and comment via `toml_edit`. Collects skipped reserved names for the caller to report. No filesystem access — operates on strings so it is exhaustively unit-testable.

**Files:**
- Modify: `src/services/sync_codex.rs`

**Code Preview:**

```rust
// crucial: mutate an existing toml_edit::Document so comments/other keys survive.
pub(crate) struct MergeOutcome {
    pub rendered: String,
    pub skipped_reserved: Vec<String>,
}

/// Merge cc-profile profiles into existing Codex TOML text (empty string = new file).
pub(crate) fn merge_codex_config(existing: &str, config: &Config) -> Result<MergeOutcome> {
    let mut doc: toml_edit::DocumentMut = existing.parse().context("Invalid TOML in Codex config")?;
    // Guard: a pre-existing non-table `model_providers` must Err, not panic on index.
    if let Some(item) = doc.get("model_providers") {
        if !item.is_table_like() {
            bail!("Codex config `model_providers` is not a table");
        }
    }
    let mut skipped = Vec::new();
    for (name, profile) in &config.profiles {
        if is_reserved_provider_id(name) {
            skipped.push(name.clone());
            continue;
        }
        // Overwrite only these three managed keys; any hand-added sub-key is preserved.
        let table = &mut doc["model_providers"][name.as_str()];
        table["name"] = toml_edit::value(name.as_str());
        table["base_url"] = toml_edit::value(profile.endpoint.as_str());
        // http_headers is an inline table: { Authorization = "Bearer <key>" }
        let mut headers = toml_edit::InlineTable::new();
        headers.insert("Authorization", format!("Bearer {}", profile.api_key).into());
        table["http_headers"] = toml_edit::value(headers);
    }
    Ok(MergeOutcome { rendered: doc.to_string(), skipped_reserved: skipped })
}
```

**Steps (run by implementer):**
1. Write failing tests:
   - Empty input → produces `[model_providers.<name>]` blocks for every profile with the three expected keys and a `Bearer ` prefix on the header.
   - Existing config with a foreign `[model_providers.other]`, a top-level `model = "..."`, and a `# comment` → all three survive verbatim; only the matching provider is overwritten.
   - Overwrite semantics (**locked: preserve unmanaged keys**): a pre-existing `[model_providers.<name>]` with a stale `base_url` and a hand-added key (e.g. `wire_api = "chat"`) gets only `name`/`base_url`/`http_headers` overwritten; the hand-added key survives verbatim. Assert this explicitly — sync manages exactly three keys and touches nothing else in the provider table.
   - Reserved name (`openai`) in profiles → excluded from output, returned in `skipped_reserved`.
   - Idempotency: `merge(merge(x))` equals `merge(x)`.
   - Invalid existing TOML → `Err` with a clear message.
   - **Wrong-type `model_providers`** (e.g. `model_providers = "x"` — structurally valid TOML but a string, not a table) → return `Err` with context, not a panic. Guard before mutable indexing.
2. Run tests — expect FAIL.
3. Implement `merge_codex_config` + `MergeOutcome`.
4. Run tests — expect PASS.
5. Commit: `git commit -m "feat(sync): format-preserving codex provider merge"`.

**Validation (tester):**
- All merge unit tests pass; full suite green.
- Comment/foreign-key preservation explicitly asserted.
- Idempotency test present and green.
- clippy + fmt clean.

**Phase 1 End Review:**
- `spec-reviewer` → Phase 1 goal met (pure core covers path, reserved, merge); tasks match plan; Success Criteria 2/3/4/5 have test coverage.
- `code-quality-reviewer` → module style matches sibling services; no re-exports added; no placeholders; `toml_edit` usage idiomatic.
- Fix findings: `implementer` + `tester`, max 2 iterations, then move to next phase.
- **Gate:** pass to next phase after max 2 fix iterations.

### Phase 2: Filesystem write + CLI wiring

**Goal:** Persist the merged config to `~/.codex/config.toml` with correct permissions and expose it as `cc-profile sync codex`.

#### Task 2.1: Public `sync` entry point (read → merge → write) [S after Task 1.3]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Add the public `sync(config, codex_path)` function that reads existing Codex TOML (empty if absent), calls `merge_codex_config`, creates the parent dir, writes atomically, sets `0o600` file / `0o700` dir perms, and returns the skipped-reserved list so the CLI can warn. Reuse the permission helpers' logic from `ConfigRepository` (extract or mirror `set_owner_only_permissions` / `set_owner_only_directory_permissions` from `src/config/repository.rs:125-152` — prefer mirroring locally to avoid widening the repository's public surface).

**Files:**
- Modify: `src/services/sync_codex.rs`

**Code Preview:**

```rust
// crucial: absent file = empty existing text, NOT an error.
pub fn sync(config: &Config, codex_path: &Path) -> Result<Vec<String>> {
    let existing = match std::fs::read_to_string(codex_path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).context(/* path */),
    };
    let outcome = merge_codex_config(&existing, config)?;
    // create ~/.codex (0o700), write config.toml (0o600)
    write_secure(codex_path, &outcome.rendered)?;
    Ok(outcome.skipped_reserved)
}
```

**Steps (run by implementer):**
1. Write failing unit tests (tempdir-based): absent file → created with expected content + `0o600`; parent dir created `0o700`; existing file merged; permissions tightened on an existing loose file (`0o644` → `0o600`); **pre-existing loose `~/.codex` dir tightened (`0o755` → `0o700`)** — mirror `repository.rs`'s unconditional re-tighten after `create_dir_all`, and assert the dir case, not just the file case.
2. Run tests — expect FAIL.
3. Implement `sync` + a local `write_secure` (mirror repository perms; on non-unix, perms are a no-op as in `repository.rs`).
4. Run tests — expect PASS.
5. Commit: `git commit -m "feat(sync): write merged codex config with owner-only perms"`.

**Validation (tester):**
- Unit tests cover create/merge/perms paths; full suite green.
- `#[cfg(unix)]` perm assertions mirror the repository tests' style.
- clippy + fmt clean.

#### Task 2.2: `sync codex` subcommand dispatch [S after Task 2.1]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Add a `Sync` command to the clap `Command` enum with a nested target so `cc-profile sync codex` parses, dispatch it to `sync_codex::sync`, and print a success summary plus a stderr warning line per skipped reserved profile. Add `sync_codex` to the `use crate::services::{...}` import. Structure `Sync` with a subcommand enum (`SyncTarget::Codex`) so future targets can be added without breaking the surface.

**Files:**
- Modify: `src/cli.rs` (enum variant at the `Command` block ~line 24; dispatch arm in `run()` ~line 85; import at line 5)

**Code Preview:**

```rust
#[derive(Debug, Subcommand)]
pub enum SyncTarget { Codex }

// in Command:
Sync {
    #[command(subcommand)]
    target: SyncTarget,
},

// dispatch:
Some(Command::Sync { target: SyncTarget::Codex }) => sync_codex_command(&repository),
```

```rust
fn sync_codex_command(repository: &ConfigRepository) -> Result<()> {
    let config = repository.load()?;
    let path = sync_codex::codex_config_path()?;
    let skipped = sync_codex::sync(&config, &path)?;
    for name in &skipped {
        eprintln!("Skipped profile \"{name}\": reserved Codex provider id");
    }
    println!("Synced {} provider(s) to {}", config.profiles.len() - skipped.len(), path.display());
    Ok(())
}
```

**Steps (run by implementer):**
1. Write failing integration tests in `tests/integration/sync.rs` (+ `mod sync;` in `tests/integration/main.rs`): run `sync codex` with `HOME` + `CODEX_HOME` set to tempdirs; assert `.codex/config.toml` contains `[model_providers.profile-a]`, `Bearer sk-ant-secret`, `base_url`; assert a reserved-name profile is skipped with a stderr warning and exit `0`; assert an existing foreign provider/comment survives.
2. Run tests — expect FAIL (subcommand unknown).
3. Implement enum variant, dispatch, and `sync_codex_command`.
4. Run tests — expect PASS.
5. Commit: `git commit -m "feat(cli): add sync codex subcommand"`.

**Validation (tester):**
- Integration tests green; `mod sync;` registered.
- Success stdout + skipped stderr both asserted.
- Full suite + clippy + fmt clean.

**Phase 2 End Review:**
- `spec-reviewer` → `sync codex` runs end-to-end; Success Criteria 1/4/6 covered; scope not drifted (no model/wire_api written).
- `code-quality-reviewer` → clap wiring matches existing subcommand style; error/`Result` handling idiomatic; perms mirror repository posture.
- Fix findings: `implementer` + `tester`, max 2 iterations, then move to next phase.
- **Gate:** pass to next phase after max 2 fix iterations.

### Phase 3: Interactive menu + docs

**Goal:** Surface `sync codex` in the interactive menu and document the command.

#### Task 3.1: Interactive "Sync codex" menu item [P with Task 3.2]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Add a `"Sync codex"` option to the interactive main menu and a match arm that calls a `sync_codex_flow(&repository)` helper (loads config, resolves path, calls `sync_codex::sync`, prints summary + skipped warnings). Mirror the existing `*_flow` helper pattern. Keep the `_ => unreachable!(...)` arm last. This is TTY-only code (excluded from coverage builds like the rest of `interactive.rs`).

**Files:**
- Modify: `src/interactive.rs` (options vec ~line 15-21; match arms ~line 32-41; import at line 2)

**Steps (run by implementer):**
1. Write a unit test for the extractable pure part of the flow if any (e.g. the summary string builder); the menu wiring itself is TTY-driven and covered by manual verification since `interactive.rs` is `#[cfg(not(coverage))]`.
2. Run tests — expect FAIL for the helper.
3. Implement menu option, match arm, and `sync_codex_flow`.
4. Run tests — expect PASS; manually drive the menu once (`cc-profile` → "Sync codex") against a `CODEX_HOME` tempdir.
5. Commit: `git commit -m "feat(interactive): add sync codex menu option"`.

**Validation (tester):**
- Any extracted helper is unit-tested.
- Manual TTY run confirmed and noted in evidence (menu shows item, sync writes file).
- clippy + fmt clean.

#### Task 3.2: README + command table docs [P with Task 3.1]

**Subagent:** `implementer` → `tester`

**Scope:** Document `cc-profile sync codex` in `README.md`: add a row to the Commands table and a short subsection explaining the provider mapping, inline-key placement, reserved-name skipping, and that other Codex config is preserved. No code.

**Files:**
- Modify: `README.md` (Commands table ~line 138-149; new subsection)

**Steps (run by implementer):**
1. Add the command-table row and subsection matching the doc voice of the `show-command` entry.
2. Verify no broken ToC/anchor links.
3. Commit: `git commit -m "docs: document sync codex subcommand"`.

**Validation (tester):**
- Table row present; subsection accurate to implemented behavior (inline Bearer header, no model, skip reserved).
- Markdown renders; links valid.

**Phase 3 End Review:**
- `spec-reviewer` → all Success Criteria met; docs match behavior; interactive flow matches non-interactive command.
- `code-quality-reviewer` → interactive code matches sibling flows; README voice consistent; no placeholders.
- Fix findings: `implementer` + `tester`, max 2 iterations, then done.
- **Gate:** final phase — its review is the final gate.
