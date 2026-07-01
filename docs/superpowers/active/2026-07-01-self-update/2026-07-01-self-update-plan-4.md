# Part 4 — Homebrew/Cargo Delegation and Standalone Self-Replacement

## Goal

Make `cc-profile update` perform real updates for package-manager installs and verified standalone installs.

## Task 4.1 — Add Homebrew update backend

**Files to touch**

- `src/services/update.rs`
- `src/services/install_method.rs`
- tests for update command construction and fake `brew`

**TDD steps**

1. Add tests that detected Homebrew installs call structured commands equivalent to:
   - `brew update`
   - `brew upgrade therealhieu/tap/cc-profile`
2. Use a fake `brew` executable on `PATH` in integration tests.
3. Require confirmation unless `--yes` is passed.
4. Propagate failure status with actionable output.

**Verification commands**

```bash
cargo test homebrew_update
cargo test --test integration update_homebrew -- --nocapture
```

**Commit step**

```bash
git add src/services/update.rs src/services/install_method.rs tests
git commit -m "feat(update): delegate Homebrew upgrades"
```

## Task 4.2 — Add Cargo update backend

**Files to touch**

- `src/services/update.rs`
- `src/services/install_method.rs`
- tests for fake `cargo`

**TDD steps**

1. Add tests that detected Cargo installs call structured command arguments:
   - `cargo install cc-profile --locked --force`
2. Use a fake `cargo` executable on `PATH` in integration tests.
3. Require confirmation unless `--yes` is passed.
4. Ensure Cargo update does not try to replace the current executable directly.

**Verification commands**

```bash
cargo test cargo_update
cargo test --test integration update_cargo -- --nocapture
```

**Commit step**

```bash
git add src/services/update.rs src/services/install_method.rs tests
git commit -m "feat(update): delegate Cargo reinstalls"
```

## Task 4.3 — Add standalone self-replacement backend

**Files to touch**

- `Cargo.toml`
- `Cargo.lock`
- `src/services/self_replace.rs`
- `src/services/release.rs`
- `src/services/update.rs`
- tests for standalone replacement

**TDD steps**

1. Add tests for checksum parsing and mismatch rejection.
2. Add tests rejecting tar entries with path traversal.
3. Add tests extracting only the expected `cc-profile` binary.
4. Add tests for replacement success in a temp directory.
5. Add tests for rollback when replacement fails after backup creation.
6. Add dependencies from the design: `sha2`, `tar`, `flate2`, and promote `tempfile` to normal dependency if production code uses it.
7. Implement download → verify → extract → smoke-test → replace → rollback flow.

**Verification commands**

```bash
cargo test self_replace
cargo test update_standalone
cargo clippy --all-targets --all-features -- -D warnings
```

**Commit step**

```bash
git add Cargo.toml Cargo.lock src/services/self_replace.rs src/services/release.rs src/services/update.rs tests
git commit -m "feat(update): self-replace standalone binaries"
```

## Task 4.4 — Harden update error handling

**Files to touch**

- `src/services/update.rs`
- `src/services/self_replace.rs`
- `src/services/release.rs`
- tests for failure cases

**TDD steps**

1. Add tests for offline/network timeout behavior.
2. Add tests for unsupported platform behavior.
3. Add tests for permission-denied replacement behavior.
4. Add tests confirming failed downloads and checksum failures leave the existing binary untouched.
5. Improve user-facing errors with context.

**Verification commands**

```bash
cargo test update_failure
cargo test self_replace_failure
cargo clippy --all-targets --all-features -- -D warnings
```

**Commit step**

```bash
git add src/services/update.rs src/services/self_replace.rs src/services/release.rs tests
git commit -m "fix(update): preserve binary on update failures"
```
