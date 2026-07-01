# Part 3 — Update Command Core, Install Detection, and Release Lookup

## Goal

Add `cc-profile update --check` and the shared update interfaces without yet mutating package-manager or standalone binaries.

## Task 3.1 — Add update command wiring

**Files to touch**

- `src/cli.rs`
- `src/services/mod.rs`
- `src/services/update.rs`
- `tests/integration/cli.rs`

**TDD steps**

1. Add an integration test for `cc-profile update --check`.
2. Assert the command does not require `~/.cc-profile/config.toml`.
3. Add `Command::Update { check, yes }` to `src/cli.rs`.
4. Dispatch update before constructing or using profile config where possible.
5. Implement a temporary no-op update service response if release lookup is not complete yet.

**Verification commands**

```bash
cargo test --test integration update -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

**Commit step**

```bash
git add src/cli.rs src/services/mod.rs src/services/update.rs tests/integration/cli.rs
git commit -m "feat(update): add update command shell"
```

## Task 3.2 — Add install method detection

**Files to touch**

- `src/services/install_method.rs`
- `src/services/mod.rs`
- `tests/integration/update.rs` or unit tests near the module

**TDD steps**

1. Write tests for fixture paths:
   - Homebrew Cellar path
   - Homebrew opt/bin path
   - `$CARGO_HOME/bin`
   - standalone receipt path
   - unknown path
2. Implement `InstallMethod` and detection helpers using `current_exe`-compatible injectable paths for tests.
3. Ensure detection never reads profile config or logs secrets.
4. Use structured process invocation only when probing `brew` or `cargo`.

**Verification commands**

```bash
cargo test install_method
cargo clippy --all-targets --all-features -- -D warnings
```

**Commit step**

```bash
git add src/services/install_method.rs src/services/mod.rs tests
git commit -m "feat(update): detect install method"
```

## Task 3.3 — Add latest-version and release lookup

**Files to touch**

- `Cargo.toml`
- `Cargo.lock`
- `src/services/release.rs`
- `src/services/update.rs`
- tests for release parsing

**TDD steps**

1. Add tests for semver comparison:
   - current
   - outdated
   - prerelease ignored unless explicitly supported later
   - malformed latest version fails closed
2. Add tests for GitHub release JSON parsing and asset selection by target triple.
3. Add dependencies from the design: `semver`, `ureq`, `serde_json`.
4. Implement latest release lookup with a small, injectable client boundary for tests.
5. Return clear outcomes: current, update available, lookup failed.

**Verification commands**

```bash
cargo test release
cargo test update
cargo clippy --all-targets --all-features -- -D warnings
```

**Commit step**

```bash
git add Cargo.toml Cargo.lock src/services/release.rs src/services/update.rs tests
git commit -m "feat(update): check latest release version"
```
