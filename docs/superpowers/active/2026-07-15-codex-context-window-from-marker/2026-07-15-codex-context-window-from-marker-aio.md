# codex-context-window-from-marker — All-in-One Plan

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

- **Issue 1: Stripping `[1m]` discards the only 1M signal for Codex.** `build_codex_command_spec_with_program` already strips a trailing `[...]` marker so Codex receives a bare model id, but it never re-applies the intended context size.
  - *Evidence:* Active profile `mmodiary-gpt` stores `opus = "gpt-5.6-sol-thinking[1m]"`. Launch becomes:
    ```
    codex -c model_provider="mmodiary-gpt" --model gpt-5.6-sol-thinking
    ```
    Confirmed at `src/services/launch.rs:94-99` (`strip_context_marker` only). `~/.codex/config.toml` may still say `model_context_window = 1000000` for a different default model (`gpt-5.6-sol`), but `start codex` overrides `--model` without also overriding the window.
  - *Solution:* Parse the trailing marker, strip it from `--model`, and when the marker maps to a known size, append `-c model_context_window=<tokens>` (Task 1.1).

- **Issue 2: Unknown/custom model ids fall back to Codex model metadata (~256–272k).** Codex looks up the stripped slug in `~/.codex/models_cache.json` / built-in catalog. Missing slugs log `Unknown model …` and use fallback model metadata.
  - *Evidence:* Local cache has no `gpt-5.6-sol` / `gpt-5.6-sol-thinking`. Common catalog default is `context_window = 272000` with `effective_context_window_percent = 95` → **258400**. Codex binary string: `"Unknown model … 0 is used. This will use fallback model metadata."` Live log: `Unknown model claude-opus-4.8-thinking`. Upstream reports of ~1M config collapsing to ~258k: [openai/codex#19185](https://github.com/openai/codex/issues/19185), [#21801](https://github.com/openai/codex/issues/21801).
  - *Solution:* Do not rely on the catalog for proxy models. Explicitly pass `model_context_window` from the marker so Codex does not need a catalog hit for window size (Task 1.1).

## Goal

When launching Codex, translate a trailing proxy context marker on `profile.opus` into a bare `--model` id plus an explicit `-c model_context_window=<tokens>` override for known markers (`1m` → 1_000_000, `256k` → 256_000).

## Non-Goals

- No change to Claude launch (`ANTHROPIC_DEFAULT_*_MODEL` keeps the marker).
- No change to `sync_codex.rs` managed keys (`name` / `base_url` / `env_key`).
- No change to how model ids are stored in `~/.cc-profile/config.toml` — markers stay at rest.
- No `model_auto_compact_token_limit` override in this change (optional follow-up if 1M sessions compact too early).
- No writing/patching `~/.codex/models_cache.json` or custom `model_catalog_json`.
- No general free-form size parser beyond the known markers listed below (unknown markers still strip from `--model`, but do **not** invent a window).
- No interactive `/status` verification harness against a live Codex TUI.

## Current State

```
profile.opus = "gpt-5.6-sol-thinking[1m]"
        │
        ▼
build_codex_command_spec_with_program
        │
        ├─ strip_context_marker → "gpt-5.6-sol-thinking"
        │
        ▼
args = [
  "-c", "model_provider=\"mmodiary-gpt\"",
  "--model", "gpt-5.6-sol-thinking"
]
        │
        ▼
Codex: unknown slug → fallback model metadata (~272k / ~256k)
```

## Expected State

```
profile.opus = "gpt-5.6-sol-thinking[1m]"
        │
        ▼
parse_context_marker(opus)
  → model = "gpt-5.6-sol-thinking"
  → window = Some(1_000_000)
        │
        ▼
args = [
  "-c", "model_provider=\"mmodiary-gpt\"",
  "--model", "gpt-5.6-sol-thinking",
  "-c", "model_context_window=1000000"
]
        │
        ▼
Codex: explicit window override, no catalog dependency for 1M
```

Marker-less opus stays as today (provider + `--model` only).

### Marker map (authoritative)

| Marker (case-insensitive) | `model_context_window` |
|---|---|
| `1m` | `1000000` |
| `256k` | `256000` |

Trailing bracket group that is not in this map: still strip from `--model` (keep current bare-id behavior), do **not** add `model_context_window`.

## Testing

- **Framework:** Rust built-in `#[test]` (cargo test), unit tests in `src/services/launch.rs` `mod tests`.
- **TDD cycle:** failing test → `cargo test` (FAIL) → implement → `cargo test` (PASS) → commit.
- **Coverage target:** every parse branch (known markers, unknown marker strip-only, no marker, empty, non-trailing bracket) plus builder integration for with/without window override.
- **Test files:** `src/services/launch.rs` (`#[cfg(test)] mod tests`). Integration `tests/integration/launch.rs` only if a fixture profile with `[1m]` is cheap to add; unit coverage is the primary gate.

## Success Criteria

- [ ] `parse_context_marker("gpt-5.6-sol-thinking[1m]")` → `("gpt-5.6-sol-thinking", Some(1_000_000))`.
- [ ] `parse_context_marker("claude-opus-4.8-thinking[256k]")` → `("claude-opus-4.8-thinking", Some(256_000))`.
- [ ] Case-insensitive markers: `[1M]` and `[256K]` map the same as lowercase.
- [ ] Unknown trailing marker e.g. `[2m]` → stripped model id, `None` window (no invented size).
- [ ] No marker / empty / non-trailing bracket behavior matches today’s strip/passthrough semantics.
- [ ] `build_codex_command_spec` with `opus = "claude-opus-4-8[1m]"` yields exact args
      `["-c", "model_provider=\"profile-a\"", "--model", "claude-opus-4-8", "-c", "model_context_window=1000000"]`
      (existing 4-arg no-window assertion is updated/replaced, not left behind).
- [ ] `build_codex_command_spec` with `opus` ending in `[256k]` yields bare `--model` **and** `-c model_context_window=256000`.
- [ ] Marker-less opus still yields only `model_provider` + `--model` (no forced window).
- [ ] Claude builder and `sync_codex.rs` behavior unchanged.
- [ ] `build_codex_command_spec*` rustdoc describes bare `--model` plus optional `-c model_context_window=<tokens>` (no longer a fixed 4-arg list).
- [ ] All tests pass; `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass.
- [ ] No placeholders remain.

## Project Standards

- Cite `/Users/hieunguyen/.config/opencode/AGENTS.md` (global): concise, correct, show code with paths.
- Cite `docs/specs/rust.md`, `docs/specs/rust_testing.md`, `docs/specs/development.md`, `docs/specs/git.md`.
- Match `src/services/launch.rs` style: private helpers with a short *why* doc comment; tests colocated in `mod tests`.
- MSRV 1.85 — plain `match`, no let-chains.
- Conventional Commits; keep the change scoped to the Codex builder path.

## Implementation Plan

**Hierarchy:** `Phase → Task → Step`

**Parallelism key:** `[P]` = parallel with siblings, `[S]` = sequential, `[S after Task X.Y]` = blocks on X.Y.

### Phase 1: Marker → Codex `model_context_window`

**Goal:** Codex launch args carry an explicit context-window override whenever `profile.opus` ends with a known proxy marker.

#### Task 1.1: `parse_context_marker` + builder wiring [S]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Replace pure strip-only handling with a private `parse_context_marker` that is the **only** trailing-bracket detector and returns `(bare_model_id, Option<u64>)`. Detection rules match today’s strip: `rfind('[')` + `ends_with(']')` (plain `match`, no let-chain). Known markers via ASCII case-insensitive compare: `1m` → `1_000_000`, `256k` → `256_000`. Wire `build_codex_command_spec_with_program` to use the bare id as `--model` and, when `Some(tokens)`, append `"-c"` and `format!("model_context_window={tokens}")` after the model args. Unknown trailing markers still strip to bare id with `None` window. Delete `strip_context_marker` or keep it only as a thin wrapper `parse_context_marker(m).0`. Migrate all existing `strip_context_marker_*` tests onto `parse_context_marker` (same inputs; assert full `(bare, Option<u64>)`). Update/replace `build_codex_command_spec_strips_context_marker_from_model` so the `[1m]` case expects the 6-arg vector including `model_context_window=1000000` — do not leave the old 4-arg no-window assertion. Update `build_codex_command_spec*` rustdoc so args are no longer described as a fixed 4-element list. Do not alter Claude builder, env map, provider quoting, or `sync_codex`.

**Files:**
- Edit: `src/services/launch.rs` (helper, `build_codex_command_spec_with_program`, unit tests in `mod tests`)
- Optional edit: `tests/integration/launch.rs` only if adding a `[1m]` fixture is trivial without widening the happy-path profile fixture for all cases

**Code Preview:** *(crucial parts only)*

```rust
/// Proxy markers like `[1m]` are not Codex model ids. Strip a trailing `[...]`
/// and, for known sizes, return the token count Codex should use via
/// `-c model_context_window=…`.
fn parse_context_marker(model: &str) -> (&str, Option<u64>) {
    let (bare, marker) = match model.rfind('[') {
        Some(i) if model.ends_with(']') => (&model[..i], Some(&model[i + 1..model.len() - 1])),
        _ => (model, None),
    };
    let window = match marker {
        Some(m) if m.eq_ignore_ascii_case("1m") => Some(1_000_000),
        Some(m) if m.eq_ignore_ascii_case("256k") => Some(256_000),
        _ => None,
    };
    (bare, window)
}

// in build_codex_command_spec_with_program:
let (model, window) = parse_context_marker(&profile.opus);
let mut args = vec![
    "-c".into(),
    format!("model_provider=\"{name}\""),
    "--model".into(),
    model.to_string(),
];
if let Some(tokens) = window {
    args.push("-c".into());
    args.push(format!("model_context_window={tokens}"));
}
```

**Steps (run by implementer):**

1. Write failing tests covering:
   - `parse_context_marker("gpt-5.6-sol-thinking[1m]")` → `("gpt-5.6-sol-thinking", Some(1_000_000))`.
   - `parse_context_marker("…[256k]")` → `Some(256_000)`; also `[1M]` / `[256K]` via `eq_ignore_ascii_case`.
   - Unknown marker `[2m]` → bare id + `None`.
   - Explicit passthrough tuples:
     - `"grok-composer-2.5-fast"` → `("grok-composer-2.5-fast", None)`
     - `""` → `("", None)`
     - `"foo[1m]-bar"` → `("foo[1m]-bar", None)`
   - **Update/replace** existing `build_codex_command_spec_strips_context_marker_from_model`:
     `opus = "claude-opus-4-8[1m]"` must assert exact args
     `["-c", "model_provider=\"profile-a\"", "--model", "claude-opus-4-8", "-c", "model_context_window=1000000"]`
     (do **not** leave the old 4-arg no-window expectation).
   - Builder case for `[256k]` → bare `--model` + `model_context_window=256000`.
   - Marker-less opus still has no `model_context_window` arg.
   - Migrate `strip_context_marker_*` unit tests onto `parse_context_marker_*` (same inputs; full `(bare, Option<u64>)`).
   - Existing env / program / provider-quote tests still hold.
2. Run `cargo test` — expect FAIL (parser absent / builder does not emit window / old 4-arg test updated to 6-arg and fails).
3. Implement `parse_context_marker` as the only trailing-bracket detector (skeleton above). Remove `strip_context_marker` or thin-wrap as `parse_context_marker(m).0`. Wire builder. Update `build_codex_command_spec*` rustdoc so args are no longer a fixed 4-element list.
4. Run `cargo test` — expect PASS. Run `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`.
5. Commit: `git commit -m "fix(codex): map [1m]/[256k] markers to model_context_window"`
   Keep this commit scoped to the marker→window change; do not mix unrelated dirty-tree work (`env_key` / `CC_PROFILE_API_KEY`) into it.

**Validation (tester):**
- Full suite passes (unit + `tests/integration/*`).
- All behaviors in Scope covered by tests.
- No regressions on Claude launch / sync codex tests.
- Lint + format checks pass.

**Phase 1 End Review:**
- `spec-reviewer` → Phase 1 goal met; marker map honored; Success Criteria progress; Non-Goals respected
- `code-quality-reviewer` → style, standards, no placeholders, single source of truth for trailing-bracket logic
- Fix findings: `implementer` + `tester`, max 2 iterations, then done
- **Gate:** final after max 2 fix iterations
