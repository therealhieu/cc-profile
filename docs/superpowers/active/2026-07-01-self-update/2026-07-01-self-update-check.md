# Check — cc-profile Self-Update

Use this checklist after implementation. Do not mark an item complete until verified.

## Review

- [ ] All tasks in `2026-07-01-self-update-plan-1.md` are complete.
- [ ] All tasks in `2026-07-01-self-update-plan-2.md` are complete.
- [ ] All tasks in `2026-07-01-self-update-plan-3.md` are complete.
- [ ] All tasks in `2026-07-01-self-update-plan-4.md` are complete.
- [ ] All tasks in `2026-07-01-self-update-plan-5.md` are complete.
- [ ] `cc-profile --version` prints the package version.
- [ ] `cc-profile update --check` works without reading or mutating profile config.
- [ ] `cc-profile update` delegates Homebrew installs through Homebrew.
- [ ] `cc-profile update` delegates Cargo installs through Cargo.
- [ ] `cc-profile update` self-replaces standalone installs only after checksum verification.
- [ ] Failed standalone updates leave the current binary usable.
- [ ] Package-manager installs are not overwritten directly by the self-replacement backend.
- [ ] Passive update notice only runs in interactive mode and never installs updates.
- [ ] `CC_PROFILE_NO_UPDATE_CHECK=1` disables passive checks.
- [ ] README documents Homebrew, Cargo, standalone install, update, uninstall, and troubleshooting.

## Decisions

- [ ] Rust edition is `2024` and `rust-version` is `1.85`.
- [ ] Package metadata includes description, license, repository, homepage, readme, keywords, and categories.
- [ ] The integration-test shim is not installed as a production binary.
- [ ] GitHub Release assets use the design's archive names and include `SHA256SUMS`.
- [ ] Homebrew formula builds from source first; bottles can be added later.
- [ ] Standalone self-update rejects checksum mismatches and path traversal archive entries.
- [ ] `brew` and `cargo` are invoked with structured command arguments, not shell strings.

## Risks

- [ ] Network failures produce actionable errors and leave the binary untouched.
- [ ] Permission failures produce actionable errors and leave the binary untouched.
- [ ] Malformed release metadata fails closed.
- [ ] Unsupported platforms print manual install instructions.
- [ ] No update code sends profile config, API keys, endpoint URLs, model names, or custom env vars.
- [ ] Rollback behavior is tested for standalone replacement failures.

## Local CI

- [ ] `./scripts/ci.sh` passes.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo package --list` shows expected package contents.
- [ ] `cargo publish --dry-run` passes on a clean tree.

## Release and PR

- [ ] PR is open.
- [ ] GitHub Actions CI is green.
- [ ] Release workflow has been tested or reviewed with required secrets documented.
- [ ] Homebrew formula has been tested locally or marked with exact reason it could not run.
