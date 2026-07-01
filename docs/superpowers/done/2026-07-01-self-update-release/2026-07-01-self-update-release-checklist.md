# Release Checklist — cc-profile Self-Update

Implementation and local verification are complete (see `../2026-07-01-self-update/2026-07-01-self-update-check.md`).
Everything below is **release execution**: merging, tagging, publishing, and finishing the Homebrew tap.
Do not tick an item until it is verified. Steps are ordered — later steps depend on earlier ones.

**Status: all steps complete. Released as `v0.1.0` on 2026-07-01.**

## Preconditions (one-time secrets & access)

- [x] `CARGO_REGISTRY_TOKEN` secret is set in the GitHub repo (Settings → Secrets → Actions).
      Get it from crates.io → Account Settings → API Tokens. Without it, `release.yml` fails the publish step.
- [x] The crate name `cc-profile` is available (or already owned) on crates.io.
      Check: `cargo search cc-profile`. *(Was unclaimed; now published & owned.)*
- [x] `gh` CLI is authenticated locally: `gh auth status`. *(therealhieu; scopes: repo, workflow.)*

## 1. Merge `init-design` → `master`

- [x] Working tree is clean: `git status`.
- [x] Open the PR. *(PR #1 — "Add cc-profile self-update and release workflow".)*
- [x] CI (`ci.yml`) is green on the PR. *(Rust verification pass.)*
- [x] Merge the PR. *(Merge commit `a085066`.)*
- [x] Locally sync master.

## 2. Confirm the release version

- [x] `Cargo.toml` `version` matches the tag (`0.1.0`).
- [x] Final clean-tree gate passes locally: fmt / clippy / test (112 pass) / publish --dry-run.

## 3. Tag & trigger the release (⚠️ irreversible once live)

- [x] Push the tag `v0.1.0`. *(Re-tagged onto fixed master after workflow bug — see note.)*
- [x] `release.yml` completes green. *(Run `28494862489`, all 7 jobs green.)*
- [x] GitHub Release `v0.1.0` exists with these assets:
      - [x] `cc-profile-v0.1.0-aarch64-apple-darwin.tar.gz`
      - [x] `cc-profile-v0.1.0-x86_64-apple-darwin.tar.gz`
      - [x] `cc-profile-v0.1.0-x86_64-unknown-linux-gnu.tar.gz`
      - [x] `SHA256SUMS`
- [x] Crate published: crates.io `cc-profile` `0.1.0` → HTTP 200.

> **Workflow bug fixed mid-release (PR #2):** the original `release` job downloaded build
> artifacts into the repo checkout, dirtying the tree, so `cargo publish --locked` refused to
> publish. Split crates.io publish into its own job with a clean checkout (no artifact download)
> and dropped the deprecated `--token` flag. First tag's GitHub Release was deleted and the tag
> re-pushed onto fixed master for one clean green run. No crates.io version was ever burned.

## 4. Verify Cargo & standalone install paths (post-release)

- [x] `cargo install cc-profile --locked` succeeds; `cc-profile --version` → `0.1.0`; only `cc-profile` binary installed.
- [x] `cc-profile update --check` returns cleanly (no more 404).
- [x] Standalone `curl … install.sh | sh` succeeds; version `0.1.0`; receipt `method = "standalone"` written.

> **`install.sh` bugs fixed (PR #3):** `curl | sh` was completely broken under `sh`+`set -u`.
> Three fixes: (1) `${BASH_SOURCE[0]}` source-guard → `${BASH_SOURCE[0]:-${0}}`;
> (2) archive downloaded as `archive.tar.gz` never matched the `SHA256SUMS` key → download to
> the real asset name; (3) `local tmp` was unbound at the `EXIT` trap's global scope → promoted
> to a script-global `CC_PROFILE_TMP`. shellcheck clean; mapping test still passes.

## 5. Homebrew tap (manual — NOT automated by `release.yml`)

- [x] Created **public** repo `therealhieu/homebrew-tap`.
- [x] Computed the source-tarball SHA. *(Note: flipping the repo public regenerated GitHub's
      auto-tarball; final stable SHA is `eb4dc392…`, not the earlier private-repo `2f641d5…`.)*
- [x] Copied `Formula/cc-profile.rb` into `homebrew-tap/Formula/cc-profile.rb` with the real SHA.
- [x] Committed & pushed the tap repo.
- [x] Verify locally:
      - [x] `brew tap therealhieu/tap`
      - [x] `brew install therealhieu/tap/cc-profile` (built from source)
      - [x] `brew test cc-profile`
      - [x] `brew audit --strict --online therealhieu/tap/cc-profile` *(after fixing `livecheck`-before-`depends_on` ordering, PR #4)*
      - [x] `cc-profile --version` → `0.1.0`
      - [x] `cc-profile update --check` detects Homebrew.

## 6. Finalize docs & tracking

- [x] Back-filled the two open Homebrew items in `../2026-07-01-self-update/2026-07-01-self-update-check.md`.
- [x] Confirmed README install/update/uninstall commands match published reality (removed future-tense "once the tap is published" phrasing).
- [ ] Move `active/2026-07-01-self-update*` docs to `done/` per workflow convention.

## Open decisions (optional, not blockers)

- [ ] **Automate the tap bump**: add a step to `release.yml` that opens a PR against `homebrew-tap`
      with the new version + SHA on each release, so step 5 stops being manual.
      Needs a token scoped to the tap repo only. *(Extra sharp edge: repo-visibility changes can
      regenerate the source-tarball SHA — automation should compute it at release time, not reuse a cached value.)*
- [ ] **Prebuilt bottles**: current formula builds from source (`depends_on "rust"`).
      Add bottles later for faster user installs.
