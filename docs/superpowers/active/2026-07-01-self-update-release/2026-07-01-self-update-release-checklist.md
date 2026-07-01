# Release Checklist — cc-profile Self-Update

Implementation and local verification are complete (see `../2026-07-01-self-update/2026-07-01-self-update-check.md`).
Everything below is **release execution**: merging, tagging, publishing, and finishing the Homebrew tap.
Do not tick an item until it is verified. Steps are ordered — later steps depend on earlier ones.

## Preconditions (one-time secrets & access)

- [x] `CARGO_REGISTRY_TOKEN` secret is set in the GitHub repo (Settings → Secrets → Actions).
      Get it from crates.io → Account Settings → API Tokens. Without it, `release.yml` fails the publish step.
- [x] The crate name `cc-profile` is available (or already owned) on crates.io.
      Check: `cargo search cc-profile`. *(crates.io API → HTTP 404 = unclaimed.)*
- [x] `gh` CLI is authenticated locally: `gh auth status`. *(therealhieu; scopes: repo, workflow.)*

## 1. Merge `init-design` → `master`

- [ ] Working tree is clean: `git status`.
- [ ] Open the PR: `gh pr create --base master --head init-design --title "feat: self-update + publishing" --fill`.
- [ ] CI (`ci.yml`) is green on the PR.
- [ ] Merge the PR.
- [ ] Locally sync master: `git checkout master && git pull`.

## 2. Confirm the release version

- [ ] `Cargo.toml` `version` matches the tag you intend to push (currently `0.1.0`).
      `release.yml` `validate` job **fails** if `v<tag>` ≠ `Cargo.toml` version.
- [ ] Final clean-tree gate passes locally:
      `cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --workspace && cargo publish --dry-run`.

## 3. Tag & trigger the release (⚠️ irreversible once live)

- [ ] Push the tag: `git tag v0.1.0 && git push origin v0.1.0`.
- [ ] `release.yml` completes green. Watch: `gh run watch`.
- [ ] GitHub Release `v0.1.0` exists with these assets:
      - [ ] `cc-profile-v0.1.0-aarch64-apple-darwin.tar.gz`
      - [ ] `cc-profile-v0.1.0-x86_64-apple-darwin.tar.gz`
      - [ ] `cc-profile-v0.1.0-x86_64-unknown-linux-gnu.tar.gz`
      - [ ] `SHA256SUMS`
- [ ] Crate published: `cargo search cc-profile` shows `0.1.0`.

## 4. Verify Cargo & standalone install paths (post-release)

- [ ] `cargo install cc-profile --locked` succeeds in a clean shell; `cc-profile --version` → `0.1.0`.
- [ ] `cc-profile update --check` now returns cleanly (no more 404 — a release exists).
- [ ] Standalone: `curl -fsSL https://raw.githubusercontent.com/therealhieu/cc-profile/master/install.sh | sh`,
      then `cc-profile --version`.

## 5. Homebrew tap (manual — NOT automated by `release.yml`)

- [ ] Create a **separate** repo named exactly `therealhieu/homebrew-tap`
      (the `homebrew-` prefix is required for `brew tap therealhieu/tap` to resolve).
- [ ] Compute the source-tarball SHA (only exists after step 3's tag push):
      `curl -sL https://github.com/therealhieu/cc-profile/archive/refs/tags/v0.1.0.tar.gz | shasum -a 256`.
- [ ] Copy `Formula/cc-profile.rb` from this repo into `homebrew-tap/Formula/cc-profile.rb`
      and replace the placeholder `sha256` with the value above.
- [ ] Commit & push the tap repo.
- [ ] Verify locally:
      - [ ] `brew tap therealhieu/tap`
      - [ ] `brew install cc-profile` (builds from source — needs Rust toolchain)
      - [ ] `brew test cc-profile`
      - [ ] `brew audit --strict --online cc-profile`
      - [ ] `cc-profile --version` → `0.1.0`
      - [ ] `cc-profile update --check` detects Homebrew and delegates correctly.

## 6. Finalize docs & tracking

- [ ] Back-fill the two open Homebrew items in
      `../2026-07-01-self-update/2026-07-01-self-update-check.md` (now that the tap is exercised).
- [ ] Confirm README install/update/uninstall commands match the published reality.
- [ ] Move `active/2026-07-01-self-update*` docs to a `done/` (or archive) location per your workflow convention.

## Open decisions (optional, not blockers)

- [ ] **Automate the tap bump**: add a step to `release.yml` that opens a PR against `homebrew-tap`
      with the new version + SHA on each release, so step 5 stops being manual.
      Needs a token scoped to the tap repo only.
- [ ] **Prebuilt bottles**: current formula builds from source (`depends_on "rust"`).
      Add bottles later for faster user installs.
