# homebrew-release-automation — All-in-One Plan

> Concise design + implementation plan in one file. TDD. Subagent-friendly.

## Table of Contents

- [Problem](#problem)
- [Goal](#goal)
- [Non-Goals](#non-goals)
- [Current State](#current-state)
- [Expected State](#expected-state)
- [Design Decisions](#design-decisions)
- [Testing](#testing)
- [Success Criteria](#success-criteria)
- [Project Standards](#project-standards)
- [Implementation Plan](#implementation-plan)

## Problem

Cutting a release requires manual, error-prone Homebrew maintenance: the tap formula's `url` + `sha256` must be hand-edited in a separate repo, and a second reference formula in this repo (`Formula/cc-profile.rb`) has already drifted to `v0.1.0`. The tap also builds from source with `cargo install`, forcing every user to compile Rust on install even though `release.yml` already produces prebuilt archives.

## Goal

Make a release fully automated after `bump Cargo.toml + push tag`: the tap formula is regenerated from prebuilt-archive checksums and pushed automatically, from a single in-repo template that is the sole source of truth.

## Non-Goals

- Submitting to `homebrew-core` — repo has 0 stars/forks/watchers, far below the 3×-for-self-submitted notability bar (≥225 stars). Revisit after real adoption.
- True Homebrew bottles with a custom `root_url` build pipeline — overkill for a personal tap; prebuilt-archive download gives the same install speed with no extra machinery.
- Opening a PR to the tap for human merge — a solo tap uses direct push gated by `brew style`; PR-based flow is a possible later change, not this plan.
- Changing crates.io publishing or the existing `validate`/`verify`/`build`/`release` jobs.

## Current State

```
Release today:
  bump Cargo.toml + Cargo.lock ──▶ push tag vX.Y.Z ──▶ release.yml
                                                          ├─ validate (tag==Cargo.toml)
                                                          ├─ verify (ci.sh)
                                                          ├─ build (mac arm64/x86, linux x86) ─▶ *.tar.gz + SHA256SUMS
                                                          ├─ release (GitHub Release upload)
                                                          └─ publish (crates.io)

  Homebrew (MANUAL, separate repo):
    therealhieu/homebrew-tap/Formula/cc-profile.rb   ← hand-edit url+sha256, cargo install (slow)
    cc-profile/Formula/cc-profile.rb                 ← DRIFTED, pinned v0.1.0, nothing updates it
```

## Expected State

```
Release after this plan:
  bump Cargo.toml + Cargo.lock ──▶ push tag vX.Y.Z ──▶ release.yml
                                                          ├─ ... (unchanged jobs) ...
                                                          ├─ release (GitHub Release upload)
                                                          └─ bump-formula (NEW)
                                                               ├─ gh release download SHA256SUMS
                                                               ├─ render-formula.sh <ver> SHA256SUMS ─▶ cc-profile.rb
                                                               ├─ brew style (gate)
                                                               └─ push to homebrew-tap/Formula/cc-profile.rb (PAT)

  Single source of truth:
    cc-profile/packaging/homebrew/cc-profile.rb.tmpl  ← template (prebuilt archives, on_macos/on_linux)
    therealhieu/homebrew-tap/Formula/cc-profile.rb    ← GENERATED artifact, never hand-edited

  Install: brew install therealhieu/tap/cc-profile  → downloads prebuilt archive (seconds, no compile)
```

## Design Decisions

| Decision | Choice | Rationale |
|---|---|---|
| One formula or two | Template in this repo = source of truth; tap file generated | Kills drift; `bump-formula` overwrites the tap file every release |
| Bump mechanism | Custom `render-formula.sh` from `SHA256SUMS` in `release.yml` | `SHA256SUMS` already has all 3 platform hashes; deterministic; avoids `dawidd6/action-homebrew-bump-formula` multi-platform-sha256 friction |
| Formula install method | Download prebuilt per-platform archives (`on_macos`/`on_arm`/`on_intel`/`on_linux`) | Reuses existing build artifacts; seconds vs multi-minute `cargo install` |
| Tap update style | Direct push, gated by `brew style` before push | Solo tap; full automation; lint gate prevents pushing an invalid formula |
| Cross-repo auth | Fine-grained PAT `HOMEBREW_TAP_TOKEN` (contents:write on homebrew-tap) | `GITHUB_TOKEN` cannot push to a different repo |
| Pre-release local validation | Unit tests on render script + `brew style` on rendered output | A prebuilt-archive formula can't be installed before the release archive exists; validate the generator, not a live install |

## Testing

- **Framework:** bash test script (mirrors existing `tests/install_platform_mapping_test.sh`), run via `cargo test` is N/A here — invoked directly and wired into `scripts/ci.sh` is out of scope; run through the test file directly.
- **TDD cycle:** write failing bash test → run (FAIL) → implement `render-formula.sh` → run (PASS) → commit.
- **Coverage target:** every platform target in `release.yml`'s matrix maps to the correct `sha256` in the rendered formula; version substitution correct; missing-hash input fails loudly.
- **Test files:** `tests/render_formula_test.sh`, fixture `tests/fixtures/SHA256SUMS.sample`.

## Success Criteria

- [ ] `scripts/render-formula.sh <version> <SHA256SUMS>` emits a valid formula with correct version + all 3 platform `sha256` values in the right `on_*` blocks.
- [ ] Rendered formula passes `brew style` (stdin or temp file).
- [ ] `release.yml` has a `bump-formula` job that runs only on tag push, renders from the release's `SHA256SUMS`, and pushes to `homebrew-tap` using `HOMEBREW_TAP_TOKEN`.
- [ ] In-repo `Formula/cc-profile.rb` is removed; `packaging/homebrew/cc-profile.rb.tmpl` is the source of truth.
- [ ] README "Homebrew formula" section reflects generated-from-template + prebuilt-archive install.
- [ ] `tests/render_formula_test.sh` passes.
- [ ] Bootstrap steps (add PAT secret, first tap push) are documented and runnable.
- [ ] No placeholders remain.

## Project Standards

- `AGENTS.md` (root): 25B pointer — no material conventions. Global user standards apply: concise, correct, tables/arrows over prose.
- Existing shell-test precedent: `tests/install_platform_mapping_test.sh` (pure bash, `set -euo pipefail`, function-per-case). Match its style.
- Existing workflow style: `release.yml` uses `set -euo pipefail` in every `run:` block, `${GITHUB_REF_NAME}` for the tag. Match it.
- Do not restate these; cite.

## Implementation Plan

**Hierarchy:** `Phase → Task → Step`

**Parallelism key:** `[P]` = parallel with siblings, `[S]` = sequential, `[S after Task X.Y]` = blocks on X.Y.

### Phase 1: Formula template + render logic

**Goal:** A single-source-of-truth template and a tested script that renders a valid prebuilt-archive formula from `SHA256SUMS`.

#### Task 1.1: Formula template [S]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Create `packaging/homebrew/cc-profile.rb.tmpl`, a prebuilt-archive Homebrew formula with placeholders for version and the three platform sha256 values. It downloads per-platform release archives (no `cargo install`), installs the `cc-profile` binary, keeps the `livecheck` github_latest block and the `--version` test. It does NOT contain real hashes (those come from the renderer) and is not itself installable.

**Files:**
- Create: `packaging/homebrew/cc-profile.rb.tmpl`

**Code Preview:**

```ruby
# crucial: prebuilt-archive layout the renderer fills; placeholders are literal tokens
class CcProfile < Formula
  desc "Profile management for Claude Code endpoints and models"
  homepage "https://github.com/therealhieu/cc-profile"
  version "__VERSION__"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/therealhieu/cc-profile/releases/download/v__VERSION__/cc-profile-v__VERSION__-aarch64-apple-darwin.tar.gz"
      sha256 "__SHA_DARWIN_ARM64__"
    end
    on_intel do
      url "https://github.com/therealhieu/cc-profile/releases/download/v__VERSION__/cc-profile-v__VERSION__-x86_64-apple-darwin.tar.gz"
      sha256 "__SHA_DARWIN_X86_64__"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/therealhieu/cc-profile/releases/download/v__VERSION__/cc-profile-v__VERSION__-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "__SHA_LINUX_X86_64__"
    end
  end

  livecheck do
    url :stable
    strategy :github_latest
  end

  def install
    bin.install "cc-profile"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/cc-profile --version")
  end
end
```

**Steps (run by implementer):**

1. Write a failing test in `tests/render_formula_test.sh` asserting the template file exists and contains each of the four placeholder tokens (`__VERSION__`, `__SHA_DARWIN_ARM64__`, `__SHA_DARWIN_X86_64__`, `__SHA_LINUX_X86_64__`) and no hardcoded `v0.1.0`.
2. Run test — expect FAIL (file absent).
3. Create the template as previewed.
4. Run test — expect PASS.
5. Commit: `git commit -m "feat(homebrew): add prebuilt-archive formula template"`

**Validation (tester):**
- Template contains all four placeholders and no real sha256/version.
- `on_macos`/`on_arm`/`on_intel`/`on_linux` structure present.
- No regressions in existing tests.

#### Task 1.2: render-formula.sh + tests [S after Task 1.1]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Create `scripts/render-formula.sh <version> <sha256sums-path>` that reads `SHA256SUMS` (lines `<sha>  cc-profile-v<ver>-<target>.tar.gz`), maps each of the three known targets to its placeholder, substitutes into the template, and writes the rendered formula to stdout. It must fail with a nonzero exit and a clear message if any of the three target hashes is missing. It does NOT push, lint, or touch the tap.

**Files:**
- Create: `scripts/render-formula.sh`
- Create: `tests/render_formula_test.sh`
- Create: `tests/fixtures/SHA256SUMS.sample`

**Code Preview:**

```bash
# crucial: target → placeholder mapping and fail-on-missing contract
declare -A MAP=(
  [aarch64-apple-darwin]="__SHA_DARWIN_ARM64__"
  [x86_64-apple-darwin]="__SHA_DARWIN_X86_64__"
  [x86_64-unknown-linux-gnu]="__SHA_LINUX_X86_64__"
)
for target in "${!MAP[@]}"; do
  file="cc-profile-v${version}-${target}.tar.gz"
  sha="$(awk -v f="${file}" '$2==f{print $1; exit}' "${sums}")"
  [[ -z "${sha}" ]] && { echo "missing sha256 for ${file}" >&2; exit 1; }
  rendered="${rendered//${MAP[$target]}/${sha}}"
done
rendered="${rendered//__VERSION__/${version}}"
printf '%s\n' "${rendered}"
```

**Steps (run by implementer):**

1. Write failing tests in `tests/render_formula_test.sh`:
   - Given `SHA256SUMS.sample` (3 valid target lines) + version `1.2.3`, rendered output contains version `1.2.3`, contains each of the 3 sample hashes, and contains no remaining `__...__` placeholder.
   - Given a `SHA256SUMS` missing the linux target, the script exits nonzero.
2. Run tests — expect FAIL (script absent).
3. Implement `scripts/render-formula.sh` (`set -euo pipefail`, resolve template path relative to script, mapping + substitution as previewed).
4. Run tests — expect PASS.
5. Commit: `git commit -m "feat(homebrew): render formula from SHA256SUMS"`

**Validation (tester):**
- Both test cases pass; run `tests/render_formula_test.sh` and capture output.
- `bash -n scripts/render-formula.sh` clean; `shellcheck` clean if available.
- No remaining placeholder in a successful render; missing-hash path exits nonzero.

**Phase 1 End Review:**
- `spec-reviewer` → Phase 1 goal met (template + tested renderer); matches Design Decisions; Success Criteria 1 + 6 progress.
- `code-quality-reviewer` → shell style matches `tests/install_platform_mapping_test.sh`, `set -euo pipefail`, no placeholders, no hardcoded paths.
- Fix findings: `implementer` + `tester`, max 2 iterations, then move to next phase.
- **Gate:** pass to next phase after max 2 fix iterations.

### Phase 2: Release automation

**Goal:** `release.yml` regenerates and pushes the tap formula automatically on every tag release, gated by a formula lint.

#### Task 2.1: bump-formula job in release.yml [S after Task 1.2]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Add a `bump-formula` job to `.github/workflows/release.yml` that `needs: release`, downloads the release's `SHA256SUMS`, renders the formula via `scripts/render-formula.sh`, runs `brew style` on the rendered file as a gate, and pushes it to `therealhieu/homebrew-tap/Formula/cc-profile.rb` using `HOMEBREW_TAP_TOKEN`. It commits only when content changed. It does NOT alter existing jobs or crates.io publishing.

**Files:**
- Modify: `.github/workflows/release.yml`

**Code Preview:**

```yaml
# crucial: needs the release, gates on brew style, cross-repo push via PAT
  bump-formula:
    name: Bump Homebrew tap formula
    needs: release
    runs-on: macos-latest        # brew preinstalled
    steps:
      - uses: actions/checkout@v4
      - name: Download SHA256SUMS
        env: { GH_TOKEN: "${{ github.token }}" }
        run: gh release download "${GITHUB_REF_NAME}" -p SHA256SUMS -D .
      - name: Render formula
        run: |
          set -euo pipefail
          version="${GITHUB_REF_NAME#v}"
          scripts/render-formula.sh "${version}" SHA256SUMS > cc-profile.rb
      - name: Lint formula (gate)
        run: brew style cc-profile.rb
      - name: Push to tap
        env: { GH_TOKEN: "${{ secrets.HOMEBREW_TAP_TOKEN }}" }
        run: |
          set -euo pipefail
          git clone "https://x-access-token:${GH_TOKEN}@github.com/therealhieu/homebrew-tap.git" tap
          cp cc-profile.rb tap/Formula/cc-profile.rb
          cd tap
          git config user.name "github-actions[bot]"
          git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
          git add Formula/cc-profile.rb
          git diff --cached --quiet || git commit -m "cc-profile ${GITHUB_REF_NAME#v}"
          git push
```

**Steps (run by implementer):**

1. Write a failing check: a lightweight assertion (grep/yaml-lint in `tests/render_formula_test.sh` or a new `tests/release_workflow_test.sh`) that `release.yml` contains a `bump-formula:` job with `needs: release`, a `brew style` step, and references `HOMEBREW_TAP_TOKEN` + `render-formula.sh`.
2. Run test — expect FAIL (job absent).
3. Add the job as previewed.
4. Run test — expect PASS. Validate YAML parses (`yq`/python `yaml.safe_load` on the file).
5. Commit: `git commit -m "ci(release): auto-bump homebrew tap formula on release"`

**Validation (tester):**
- Structural test passes; `release.yml` parses as valid YAML.
- Job ordering: `bump-formula` depends on `release`; other jobs untouched (diff limited to the new job).
- No secret value is echoed; token used only in remote URL.
- Confirm (by reading) the job is tag-gated (inherits the `on: push: tags` trigger; add an `if: startsWith(github.ref, 'refs/tags/')` guard since `workflow_dispatch` has no tag).

**Phase 2 End Review:**
- `spec-reviewer` → Success Criteria 3 met; matches Design Decisions (render-from-SHA256SUMS, direct push, style gate); tag-gating correct.
- `code-quality-reviewer` → workflow style matches existing jobs, secret handling safe, no `workflow_dispatch` crash path.
- Fix findings: `implementer` + `tester`, max 2 iterations, then move to next phase.
- **Gate:** pass to next phase after max 2 fix iterations.

### Phase 3: Cleanup, docs, and bootstrap

**Goal:** Remove the drifted in-repo formula, document the new flow, and provide runnable bootstrap steps (PAT + first tap push).

#### Task 3.1: Remove stale formula + update README [P]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Delete `Formula/cc-profile.rb` (the drifted reference). Rewrite the README "Homebrew formula" section to state that the tap formula is generated from `packaging/homebrew/cc-profile.rb.tmpl` on release and installs prebuilt archives, replacing the stale `brew install --build-from-source ./Formula/cc-profile.rb` validation snippet with `brew style` on a rendered formula + post-release `brew install therealhieu/tap/cc-profile`. Does NOT touch historical docs under `docs/superpowers/done/`.

**Files:**
- Delete: `Formula/cc-profile.rb`
- Modify: `README.md` (section at ~`README.md:157`)

**Steps (run by implementer):**

1. Write a failing test asserting `Formula/cc-profile.rb` is absent and README no longer contains `build-from-source ./Formula/cc-profile.rb`.
2. Run test — expect FAIL.
3. Delete file; rewrite README section (cite template path + prebuilt-archive install).
4. Run test — expect PASS.
5. Commit: `git commit -m "docs(homebrew): generated-from-template formula, drop stale reference"`

**Validation (tester):**
- File absent; README section accurate and internally consistent (no dangling `Formula/` link).
- Existing tests still pass.

#### Task 3.2: Bootstrap runbook [P]

**Subagent:** `implementer` (TDD) → `tester` (validate)

**Scope:** Add a short bootstrap runbook (a `## Homebrew release automation` subsection in README or a `docs/` note) covering the one-time manual steps: create a fine-grained PAT with contents:write on `therealhieu/homebrew-tap`, add it as the `HOMEBREW_TAP_TOKEN` repo secret (`gh secret set`), and perform the first tap formula push (either wait for the next tagged release, or hand-render once via `scripts/render-formula.sh` against the latest release's SHA256SUMS and push). Also add the recommended safety-net: a scheduled `brew livecheck` workflow to add to the tap repo (provided as a copy-paste block, since it lives in the other repo). Does NOT add crates that require secrets to exist at build time.

**Files:**
- Modify: `README.md` (or create `docs/homebrew-automation.md` and link it)

**Code Preview:**

```bash
# crucial: one-time secret setup + first push (runbook content, not executed by CI)
gh secret set HOMEBREW_TAP_TOKEN --repo therealhieu/cc-profile   # paste fine-grained PAT
# first push (if not waiting for next release):
gh release download "$(gh release view --json tagName -q .tagName)" -p SHA256SUMS -D /tmp
scripts/render-formula.sh "$(gh release view --json tagName -q .tagName | sed 's/^v//')" /tmp/SHA256SUMS
```

**Steps (run by implementer):**

1. Write a failing test asserting the runbook mentions `HOMEBREW_TAP_TOKEN`, `gh secret set`, and a livecheck safety-net block.
2. Run test — expect FAIL.
3. Write the runbook + tap livecheck copy-paste workflow block.
4. Run test — expect PASS.
5. Commit: `git commit -m "docs(homebrew): bootstrap runbook + livecheck safety net"`

**Validation (tester):**
- Runbook is self-contained and commands are syntactically valid (`bash -n` where applicable).
- Livecheck block is valid YAML.

**Phase 3 End Review:**
- `spec-reviewer` → Success Criteria 4, 5, 7 met; no drift left; homebrew-core correctly excluded as Non-Goal.
- `code-quality-reviewer` → docs concise (match global standards), no placeholders, historical docs untouched.
- Fix findings: `implementer` + `tester`, max 2 iterations, then complete.
- **Gate:** last phase review is the final gate; no separate final review.
