# Check — cc-profile Self-Update

Use this checklist after implementation. Do not mark an item complete until verified.

## Review

- [x] All tasks in `2026-07-01-self-update-plan-1.md` are complete.
- [x] All tasks in `2026-07-01-self-update-plan-2.md` are complete.
- [x] All tasks in `2026-07-01-self-update-plan-3.md` are complete.
- [x] All tasks in `2026-07-01-self-update-plan-4.md` are complete.
- [x] All tasks in `2026-07-01-self-update-plan-5.md` are complete.
- [x] `cc-profile --version` prints the package version.
- [x] `cc-profile update --check` works without reading or mutating profile config.
- [x] `cc-profile update` delegates Homebrew installs through Homebrew.
- [x] `cc-profile update` delegates Cargo installs through Cargo.
- [x] `cc-profile update` self-replaces standalone installs only after checksum verification.
- [x] Failed standalone updates leave the current binary usable.
- [x] Package-manager installs are not overwritten directly by the self-replacement backend.
- [x] Passive update notice only runs in interactive mode and never installs updates.
- [x] `CC_PROFILE_NO_UPDATE_CHECK=1` disables passive checks.
- [x] README documents Homebrew, Cargo, standalone install, update, uninstall, and troubleshooting.

## Decisions

- [x] Rust edition is `2024` and `rust-version` is `1.85`.
- [x] Package metadata includes description, license, repository, homepage, readme, keywords, and categories.
- [x] The integration-test shim is not installed as a production binary.
- [x] GitHub Release assets use the design's archive names and include `SHA256SUMS`.
- [x] Homebrew formula builds from source first; bottles can be added later. *(Verified 2026-07-01: tap `therealhieu/homebrew-tap` published, `brew install therealhieu/tap/cc-profile` builds from source, `brew test` and `brew audit --strict --online` pass.)*
- [x] Standalone self-update rejects checksum mismatches and path traversal archive entries.
- [x] `brew` and `cargo` are invoked with structured command arguments, not shell strings.

## Risks

- [x] Network failures produce actionable errors and leave the binary untouched.
- [x] Permission failures produce actionable errors and leave the binary untouched.
- [x] Malformed release metadata fails closed.
- [x] Unsupported platforms print manual install instructions.
- [x] No update code sends profile config, API keys, endpoint URLs, model names, or custom env vars.
- [x] Rollback behavior is tested for standalone replacement failures.

## Local CI

- [x] `./scripts/ci.sh` passes.
- [x] `cargo fmt --check` passes.
- [x] `cargo clippy --all-targets --all-features -- -D warnings` passes.
- [x] `cargo test --workspace` passes.
- [x] `cargo package --list` shows expected package contents.
- [x] `cargo publish --dry-run` passes on a clean tree.

## Release and PR

- [x] PR is open.
- [x] GitHub Actions CI is green.
- [x] Release workflow has been tested or reviewed with required secrets documented.
- [x] Homebrew formula has been tested locally or marked with exact reason it could not run. *(Verified 2026-07-01: `brew install therealhieu/tap/cc-profile` from source, `brew test`, `brew audit --strict --online`, and `cc-profile update --check` (Homebrew detection) all pass.)*