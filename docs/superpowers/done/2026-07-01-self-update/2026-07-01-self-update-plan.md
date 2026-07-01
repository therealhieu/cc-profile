# cc-profile Self-Update Implementation Plan

## Context

`cc-profile` needs a polished install and update story before publishing. The design requires one user-facing command, `cc-profile update`, that works for Homebrew, Cargo, and standalone installs while preserving package-manager ownership and safely self-replacing only standalone binaries.

This plan follows `docs/specs/development.md`: implementation is split into dependent part plans, each task includes files, TDD steps, verification, and a commit step.

## Part Plans

1. [`2026-07-01-self-update-plan-1.md`](./2026-07-01-self-update-plan-1.md) — Package readiness and user-facing metadata.
2. [`2026-07-01-self-update-plan-2.md`](./2026-07-01-self-update-plan-2.md) — CI, release artifacts, install script, and Homebrew formula template.
3. [`2026-07-01-self-update-plan-3.md`](./2026-07-01-self-update-plan-3.md) — Update command core, install detection, and release lookup.
4. [`2026-07-01-self-update-plan-4.md`](./2026-07-01-self-update-plan-4.md) — Homebrew/Cargo delegation and standalone self-replacement.
5. [`2026-07-01-self-update-plan-5.md`](./2026-07-01-self-update-plan-5.md) — Passive update notice, integration hardening, and final docs.

## Implementation Sequence

```text
Part 1 → Part 2 → Part 3 → Part 4 → Part 5
```

Part 1 must land first because later workflows and docs depend on package metadata, `--version`, and test-shim packaging.

Part 2 can start after Part 1 because release automation depends on stable package names, metadata, and binary layout.

Part 3 can start after Part 1 because it depends on the CLI shape and package version metadata.

Part 4 depends on Part 3 interfaces:

- `InstallMethod`
- `UpdateService`
- latest-version/release asset lookup
- update command dispatch

Part 5 depends on Parts 3 and 4 because passive notices reuse update lookup logic and must not mutate profile config.

## Parallelism

- Part 2 and Part 3 can run in parallel after Part 1 if implementers coordinate on dependency changes in `Cargo.toml`.
- Part 4 must wait for Part 3.
- Part 5 must wait for Part 4.
- Within each task, `tester` runs after `implementer`; `spec-reviewer` and `code-quality-reviewer` run in parallel after tests pass.

## Shared Interfaces

Part 3 should define the shared types consumed by later parts:

```text
src/services/install_method.rs
  InstallMethod::{Homebrew, Cargo, Standalone, Unknown}

src/services/release.rs
  LatestVersion
  ReleaseAsset
  ChecksumManifest

src/services/update.rs
  UpdateOptions { check: bool, yes: bool }
  UpdateOutcome
```

These types should avoid profile config dependencies and avoid logging secrets.

## Final Verification

Run after all parts complete:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
cargo package --list
cargo publish --dry-run
```

Homebrew verification, once formula/tap exists:

```bash
brew install --build-from-source ./Formula/cc-profile.rb
brew test cc-profile
brew audit --strict --online cc-profile
```

Release verification:

```text
Push tag vX.Y.Z
→ GitHub Release contains target archives and SHA256SUMS
→ crates.io version is published
→ Homebrew formula PR/update is created
→ standalone binary can update from previous version to tag version
```
