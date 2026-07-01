# Part 2 — CI, Release Artifacts, Install Script, and Homebrew Template

## Goal

Create the automation needed to publish verified release artifacts and provide easy install paths.

## Task 2.1 — Add local CI script and GitHub CI workflow

**Files to touch**

- `scripts/ci.sh`
- `.github/workflows/ci.yml`
- `README.md`

**TDD steps**

1. Create `scripts/ci.sh` with explicit jobs for format, clippy, tests, package listing, and publish dry-run.
2. Make the script runnable by humans and CI.
3. Add a GitHub Actions workflow that runs the same checks on PRs and pushes.
4. Document `./scripts/ci.sh` in README development instructions.

**Verification commands**

```bash
./scripts/ci.sh
bash -n scripts/ci.sh
```

**Commit step**

```bash
git add scripts/ci.sh .github/workflows/ci.yml README.md
git commit -m "ci: add Rust verification workflow"
```

## Task 2.2 — Add release workflow for GitHub assets and crates.io

**Files to touch**

- `.github/workflows/release.yml`
- `README.md`

**TDD steps**

1. Add workflow validation that tag `vX.Y.Z` matches `Cargo.toml` package version.
2. Build release binaries for:
   - `aarch64-apple-darwin`
   - `x86_64-apple-darwin`
   - `x86_64-unknown-linux-gnu`
3. Archive each binary as `cc-profile-vX.Y.Z-<target>.tar.gz` with `LICENSE` and `README.md`.
4. Generate `SHA256SUMS`.
5. Create a GitHub Release.
6. Publish to crates.io with `CARGO_REGISTRY_TOKEN`.
7. Document required secrets.

**Verification commands**

```bash
cargo test --workspace
# Validate workflow syntax via GitHub or local action linter if available.
```

Manual dry-run expectation: workflow can be reviewed without requiring publish credentials on PRs.

**Commit step**

```bash
git add .github/workflows/release.yml README.md
git commit -m "ci(release): publish archives and crate on tags"
```

## Task 2.3 — Add standalone install script

**Files to touch**

- `install.sh`
- `README.md`

**TDD steps**

1. Add shell tests or manual checks for platform-to-target mapping.
2. Implement `install.sh` to download the latest GitHub Release asset for the current platform.
3. Verify `SHA256SUMS` before installing.
4. Install to a user-writable prefix, defaulting to `~/.local/bin` unless overridden.
5. Write `~/.cc-profile/install.toml` with `method = "standalone"`.
6. Document install and uninstall commands.

**Verification commands**

```bash
bash -n install.sh
CC_PROFILE_INSTALL_DIR=/tmp/cc-profile-install ./install.sh --dry-run
```

**Commit step**

```bash
git add install.sh README.md
git commit -m "feat(install): add standalone installer"
```

## Task 2.4 — Add Homebrew formula template

**Files to touch**

- `Formula/cc-profile.rb` or `packaging/homebrew/cc-profile.rb`
- `README.md`

**TDD steps**

1. Add a source-build formula matching the design.
2. Include `desc`, `homepage`, `url`, `sha256`, `license`, Rust build dependency, install block, test block, and `livecheck`.
3. Document that the canonical formula lives in `therealhieu/homebrew-tap` once the tap exists.

**Verification commands**

```bash
brew install --build-from-source ./Formula/cc-profile.rb
brew test cc-profile
brew audit --strict --online cc-profile
```

If Homebrew is unavailable locally, record that these checks must run on a macOS machine.

**Commit step**

```bash
git add Formula/cc-profile.rb README.md
git commit -m "build(homebrew): add formula template"
```
