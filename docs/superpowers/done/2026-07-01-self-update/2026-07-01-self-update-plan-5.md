# Part 5 — Passive Update Notice, Integration Hardening, and Final Docs

## Goal

Add the optional once-per-day update notice, finish documentation, and verify the end-to-end release/update story.

## Task 5.1 — Add passive update notice in interactive mode

**Files to touch**

- `src/interactive.rs`
- `src/services/update.rs`
- `src/services/update_check_cache.rs`
- `src/services/mod.rs`
- tests for cache behavior

**TDD steps**

1. Add tests for `~/.cc-profile/update-check.toml` cache behavior:
   - no cache means eligible to check
   - recent check skips network
   - stale check performs lookup
   - `CC_PROFILE_NO_UPDATE_CHECK=1` skips lookup
2. Add cache read/write code separate from profile config.
3. Call passive check only when entering interactive mode.
4. Print notice only; never install updates from passive check.
5. Ensure passive check failures do not block interactive mode.

**Verification commands**

```bash
cargo test update_check_cache
cargo test interactive
cargo clippy --all-targets --all-features -- -D warnings
```

**Commit step**

```bash
git add src/interactive.rs src/services/update.rs src/services/update_check_cache.rs src/services/mod.rs tests
git commit -m "feat(update): show passive update notices"
```

## Task 5.2 — Complete README install/update/uninstall docs

**Files to touch**

- `README.md`

**TDD steps**

1. Add README sections for:
   - Homebrew install
   - Cargo install
   - standalone install script
   - `cc-profile update`
   - `cc-profile update --check`
   - uninstall for each method
   - troubleshooting permission and checksum failures
2. Ensure docs state that Homebrew and Cargo installs update through their package managers.
3. Ensure docs state that standalone installs verify checksums before self-replacement.

**Verification commands**

```bash
cargo test --workspace
```

Manual verification: follow README commands in a temp install location where possible.

**Commit step**

```bash
git add README.md
git commit -m "docs: document install and update workflows"
```

## Task 5.3 — End-to-end update tests

**Files to touch**

- `tests/integration/update.rs`
- `tests/integration/common.rs`
- release/update fixtures under `tests/fixtures/` if needed

**TDD steps**

1. Add integration tests for `cc-profile update --check` with fixture release metadata.
2. Add integration tests for fake Homebrew update.
3. Add integration tests for fake Cargo update.
4. Add integration tests for standalone update in a temp directory.
5. Add a test proving update commands do not read or mutate `~/.cc-profile/config.toml`.

**Verification commands**

```bash
cargo test --test integration update -- --nocapture
cargo test --workspace
```

**Commit step**

```bash
git add tests
git commit -m "test(update): cover update paths end to end"
```

## Task 5.4 — Final release readiness check

**Files to touch**

- `docs/superpowers/active/2026-07-01-self-update/2026-07-01-self-update-check.md`
- any implementation files needed for fixes found during verification

**TDD steps**

1. Run the full local CI script.
2. Run Cargo package and publish dry-run checks.
3. Verify package contents exclude planning-only docs and production install exposes only `cc-profile`.
4. Verify release workflow documentation and required secrets.
5. Mark checklist items complete only after verification.

**Verification commands**

```bash
./scripts/ci.sh
cargo package --list
cargo publish --dry-run
cargo install --path . --locked --force --root /tmp/cc-profile-install-check
find /tmp/cc-profile-install-check/bin -maxdepth 1 -type f -print
```

**Commit step**

```bash
git add docs/superpowers/active/2026-07-01-self-update/2026-07-01-self-update-check.md
git commit -m "docs(update): record release readiness checks"
```
