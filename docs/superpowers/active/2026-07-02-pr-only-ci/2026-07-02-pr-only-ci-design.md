# Design: PR-only CI

## Purpose

Make the normal CI workflow run only for pull requests, while preserving release automation for tags and manual dispatch.

## Current State

**Purpose** — Document the existing CI trigger behavior before changing it.

**Current state**

```text
push to master/main ─┐
                     ├─> .github/workflows/ci.yml ─> ./scripts/ci.sh
pull_request ───────┘

tag v* / manual ──────> .github/workflows/release.yml ─> release jobs
```

**Expected state**

```text
pull_request ─────────> .github/workflows/ci.yml ─> ./scripts/ci.sh

tag v* / manual ──────> .github/workflows/release.yml ─> release jobs
```

Today `.github/workflows/ci.yml` runs for both pushes to `master`/`main` and pull requests. `.github/workflows/release.yml` is separate and runs on version tag pushes plus `workflow_dispatch`.

## Proposed Change

**Purpose** — Remove branch push triggers from normal CI so CI runs only during pull request validation.

**Current state**

```text
.github/workflows/ci.yml
└─ on:
   ├─ push:
   │  └─ branches: master, main
   └─ pull_request:
```

**Expected state**

```text
.github/workflows/ci.yml
└─ on:
   └─ pull_request:
```

Change only `.github/workflows/ci.yml`. Remove the `push` trigger block and leave `pull_request` as the only event. Do not change jobs, steps, concurrency, `scripts/ci.sh`, or release automation.

## Release Workflow Boundary

**Purpose** — Keep publishing and release validation independent from normal PR CI.

**Current state**

```text
.github/workflows/release.yml
└─ on:
   ├─ push:
   │  └─ tags: v*
   └─ workflow_dispatch: {}
```

**Expected state**

```text
.github/workflows/release.yml
└─ on:
   ├─ push:
   │  └─ tags: v*
   └─ workflow_dispatch: {}
```

No release workflow trigger changes are included in this scope.

## Testing and Verification

**Purpose** — Verify the CI trigger change is syntactically correct and limited to the intended workflow.

**Current state**

```text
CI workflow diff ──> includes push + pull_request triggers
```

**Expected state**

```text
CI workflow diff ──> removes only push trigger from ci.yml
                 └─> release.yml unchanged
```

Verification steps:

1. Inspect `.github/workflows/ci.yml` and confirm the only event is `pull_request`.
2. Inspect git diff and confirm `.github/workflows/release.yml` is unchanged.
3. Run the local verification command used by CI, `./scripts/ci.sh`, to ensure the job body still passes after the workflow edit.

## Out of Scope

- Branch protection changes.
- Release workflow changes.
- Adding merge queue triggers.
- Changing Rust verification commands.
- Pushing to GitHub or opening a PR.
