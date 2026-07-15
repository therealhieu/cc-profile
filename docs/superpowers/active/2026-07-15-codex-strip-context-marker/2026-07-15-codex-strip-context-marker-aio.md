# codex-strip-context-marker — All-in-One Plan

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

- **Issue 1: Codex receives an unrecognized `[1m]` context marker in `--model`.** cc-profile stores model ids with a proxy-side context-length marker (e.g. `claude-opus-4.8-thinking[1m]`, `gpt-5.5[1m]`, `kr/claude-opus-4.8[1m]`). When launching Codex, `build_codex_command_spec_with_program` passes `profile.opus` verbatim as the `--model` argument.
  - *Evidence:* With `active_profile = "mmodiary"` (`opus = "claude-opus-4.8-thinking[1m]"` in `~/.cc-profile/config.toml`), the built command is:
    ```
    codex -c model_provider="mmodiary" --model claude-opus-4.8-thinking[1m]
    ```
    Codex has no notion of the `[1m]` marker (a proxy convention) and forwards the literal string as the model id. Confirmed at `src/services/launch.rs:96` (`profile.opus.clone()`).
  - *Solution:* Add a `strip_context_marker` helper that removes a trailing bracketed group `[...]` and apply it only to the Codex `--model` argument (Task 1.1). Claude's launch path (`ANTHROPIC_DEFAULT_*_MODEL` envs) and the Codex config sync (`sync_codex.rs`, which writes only `name`/`base_url`/`http_headers`) both stay untouched, since Claude/the proxy understand the marker and the sync file never carries a model id.

## Goal

Strip a trailing bracketed context marker (general `[...]`) from the model id before it is passed as Codex's `--model` argument.

## Non-Goals

- No change to the Claude launch path or its `ANTHROPIC_DEFAULT_*_MODEL` envs — Claude/the proxy consume the marker.
- No change to `src/services/sync_codex.rs` — the synced `[model_providers.<name>]` block writes only `name`, `base_url`, `http_headers`; it never carries a model id.
- No change to how model ids are stored in `~/.cc-profile/config.toml` — the marker is preserved at rest.
- Not stripping markers from `model_provider` (the provider/profile name), only from `--model`.

## Current State

```
build_codex_command_spec_with_program(config)
        │
        ▼
 resolve_active_profile → (name, profile)
        │
        ▼
 args = [ "-c", model_provider="<name>",
          "--model", profile.opus.clone() ]   ← "claude-opus-4.8-thinking[1m]"
        │
        ▼
   codex --model claude-opus-4.8-thinking[1m]   ✗ Codex can't parse [1m]
```

## Expected State

```
build_codex_command_spec_with_program(config)
        │
        ▼
 resolve_active_profile → (name, profile)
        │
        ▼
 model = strip_context_marker(&profile.opus)    ← "claude-opus-4.8-thinking"
        │
        ▼
 args = [ "-c", model_provider="<name>",
          "--model", model.to_string() ]
        │
        ▼
   codex --model claude-opus-4.8-thinking       ✓
```

## Testing

- **Framework:** Rust built-in `#[test]` (cargo test), unit tests in the existing `mod tests` of `src/services/launch.rs`.
- **TDD cycle:** failing test → `cargo test` (FAIL) → implement → `cargo test` (PASS) → commit.
- **Coverage target:** every `strip_context_marker` branch (trailing marker, no marker, empty, non-trailing bracket) plus the integration through `build_codex_command_spec`.
- **Test files:** `src/services/launch.rs` (`#[cfg(test)] mod tests`).

## Success Criteria

- [ ] `strip_context_marker("claude-opus-4.8-thinking[1m]") == "claude-opus-4.8-thinking"`.
- [ ] `strip_context_marker("kr/claude-opus-4.8[1m]") == "kr/claude-opus-4.8"`.
- [ ] `strip_context_marker("grok-composer-2.5-fast") == "grok-composer-2.5-fast"` (no marker, unchanged).
- [ ] `strip_context_marker("")` returns `""` and does not panic.
- [ ] `build_codex_command_spec` for a profile whose `opus = "claude-opus-4-8[1m]"` yields `--model claude-opus-4-8` (no `[1m]`).
- [ ] `model_provider="<name>"` argument is unchanged (marker stripping does not touch the provider name).
- [ ] `sync_codex.rs` is byte-for-byte unchanged.
- [ ] All tests pass; `cargo clippy` and `cargo fmt --check` pass.
- [ ] No placeholders remain.

## Project Standards

- Cite `/Users/hieunguyen/.config/opencode/AGENTS.md` (global): concise, correct, show code with paths.
- Match the existing style of `src/services/launch.rs` — private helpers with a short doc comment explaining the *why* (see `shell_quote` at `src/services/launch.rs:119`), tests colocated in the same file's `mod tests`.
- MSRV note in `sync_codex.rs:40` (Rust 1.85, no let-chains) applies repo-wide — the helper uses a plain `match`, no let-chains.

## Implementation Plan

**Hierarchy:** `Phase → Task → Step`

**Parallelism key:** `[P]` = parallel with siblings, `[S]` = sequential, `[S after Task X.Y]` = blocks on X.Y.

### Phase 1: Strip context marker for Codex

**Goal:** Codex's `--model` argument carries the bare model id with any trailing `[...]` marker removed, verified by unit tests.

#### Task 1.1: `strip_context_marker` helper + Codex builder wiring [S]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Add a private `strip_context_marker(&str) -> &str` helper to `src/services/launch.rs` that removes a single trailing bracketed group (`[...]`) from a model id, leaving ids without a trailing `[...]` unchanged. Wire it into `build_codex_command_spec_with_program` so the `--model` argument uses the stripped `profile.opus`. Do not alter the `model_provider="<name>"` argument, the Claude builder, or `sync_codex.rs`. The helper strips only a *trailing* bracket group that closes at the end of the string; a stray `[` without a closing `]` at the end leaves the id unchanged.

**Files:**
- Edit: `src/services/launch.rs` (add helper, change the `--model` value at `src/services/launch.rs:96`, add tests in `mod tests`)

**Code Preview:** *(crucial parts only)*

```rust
/// Codex doesn't understand the proxy's `[1m]`-style context markers, so strip a
/// single trailing bracketed group (`[...]`) from the model id; ids without one
/// are returned unchanged. Plain `match` (no let-chain) to respect MSRV 1.85.
fn strip_context_marker(model: &str) -> &str {
    match model.rfind('[') {
        Some(i) if model.ends_with(']') => &model[..i],
        _ => model,
    }
}

// in build_codex_command_spec_with_program, replacing `profile.opus.clone()`:
"--model".into(),
strip_context_marker(&profile.opus).to_string(),
```

**Steps (run by implementer):**

1. Write failing tests covering:
   - `strip_context_marker` strips a trailing `[1m]`: `"claude-opus-4.8-thinking[1m]"` → `"claude-opus-4.8-thinking"`.
   - Strips for a slashed id: `"kr/claude-opus-4.8[1m]"` → `"kr/claude-opus-4.8"`.
   - Leaves a marker-less id unchanged: `"grok-composer-2.5-fast"`.
   - Empty string returns `""` without panicking.
   - A non-trailing bracket (id not ending in `]`, e.g. `"foo[1m]-bar"`) is left unchanged.
   - Update/add a `build_codex_command_spec` test: a profile with `opus = "claude-opus-4-8[1m]"` produces args `["-c", "model_provider=\"profile-a\"", "--model", "claude-opus-4-8"]`, asserting `model_provider` is untouched.
2. Run `cargo test` — expect FAIL (helper absent / `--model` still carries `[1m]`).
3. Implement the helper and change the `--model` value at `src/services/launch.rs:96`.
4. Run `cargo test` — expect PASS.
5. Commit: `git commit -m "fix(codex): strip trailing [...] context marker from --model"`

**Validation (tester):**
- `cargo test` full suite passes (unit + `tests/integration/*`).
- All behaviors in Scope are covered: strip, no-marker passthrough, empty, non-trailing bracket, and the `build_codex_command_spec` integration asserting `model_provider` is unchanged.
- No regressions in existing `launch.rs` tests (`build_codex_command_spec_uses_active_profile_provider_and_opus`, `build_codex_command_spec_quotes_provider_name_for_toml_typing`) — update expectations only where they now assert the stripped value.
- `git diff --stat` confirms `src/services/sync_codex.rs` is unchanged.
- `cargo clippy --all-targets` clean; `cargo fmt --check` clean.

**Phase 1 End Review:**
- `spec-reviewer` → Phase 1 goal met; helper strips general `[...]`; only the Codex `--model` path changed; `sync_codex.rs` and Claude path untouched; Success Criteria satisfied.
- `code-quality-reviewer` → helper doc comment explains the *why*; matches `launch.rs` style (cf. `shell_quote`); no let-chains (MSRV 1.85); no placeholders.
- Fix findings: `implementer` + `tester`, max 2 iterations, then move on.
- **Gate:** pass after max 2 fix iterations.
