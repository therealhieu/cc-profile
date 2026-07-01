# Part 1 — Package Readiness and User-Facing Metadata

## Goal

Make the crate publishable and user-facing before adding update mechanics.

## Task 1.1 — Add package metadata and license

**Files to touch**

- `Cargo.toml`
- `LICENSE`
- `README.md`

**TDD steps**

1. Run `cargo package --list --allow-dirty` and capture the current warning about missing package metadata.
2. Add package metadata from the design:
   - `edition = "2024"`
   - `rust-version = "1.85"`
   - `description`
   - `license`
   - `repository`
   - `homepage`
   - `readme`
   - `keywords`
   - `categories`
   - `exclude` for planning-only docs and `AGENTS.md`
3. Add `LICENSE` matching the selected SPDX license.
4. Add README install/update headings, but leave detailed update behavior to later tasks.

**Verification commands**

```bash
cargo fmt --check
cargo package --list --allow-dirty
cargo publish --dry-run --allow-dirty
```

Expected result: Cargo no longer warns about missing description/license/repository/homepage metadata.

**Commit step**

```bash
git add Cargo.toml LICENSE README.md
git commit -m "chore(package): add publish metadata"
```

## Task 1.2 — Add CLI version output

**Files to touch**

- `src/cli.rs`
- `tests/integration/cli.rs`

**TDD steps**

1. Add an integration test for `cc-profile --version` expecting `cc-profile <CARGO_PKG_VERSION>`.
2. Verify it fails before implementation.
3. Add `version` to the root Clap command in `src/cli.rs`.
4. Keep existing subcommand behavior unchanged.

**Verification commands**

```bash
cargo test --test integration -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
```

**Commit step**

```bash
git add src/cli.rs tests/integration/cli.rs
git commit -m "feat(cli): expose package version"
```

## Task 1.3 — Remove production-visible test shim

**Files to touch**

- `src/bin/cc-profile-test-claude.rs`
- `tests/integration/common.rs`
- `tests/integration/launch.rs`
- `README.md`
- `Cargo.toml` if needed

**TDD steps**

1. Add or update an integration test proving the launch path still uses `CC_PROFILE_CLAUDE_BIN`.
2. Move the shim out of `src/bin` into a test-only fixture path, or compile it from test setup only.
3. Ensure `cargo install --path . --list` exposes only `cc-profile`.
4. Update README development notes so the shim is described as test-only, not a production binary.

**Verification commands**

```bash
cargo test --workspace
cargo install --path . --locked --force --root /tmp/cc-profile-install-check
find /tmp/cc-profile-install-check/bin -maxdepth 1 -type f -print
```

Expected result: only `cc-profile` appears in the install check bin directory.

**Commit step**

```bash
git add Cargo.toml README.md src tests
git commit -m "test(cli): keep launch shim out of production installs"
```
