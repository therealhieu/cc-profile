# Development Guidance
## Agent Workflow

When executing an implementation plan, agents follow these steps in order. Do not skip steps or reorder them. Each step maps to a Superpowers skill — load and follow it.

Workflow exists to produce straightforward, correct, extensible systems, not ritual compliance. Following process does not justify over-engineering, decorative abstraction, or unnecessary complexity. When two designs both satisfy the workflow, prefer the one with clearer ownership, fewer moving parts, and easier local reasoning.

### Step 0 — Design (if no plan exists yet)

> **Skill:** `superpowers:brainstorming`

Before touching any code, explore context, ask clarifying questions, propose approaches, get user approval, write a design doc to `docs/superpowers/active/YYYY-MM-DD-<feature>/YYYY-MM-DD-<feature>-design.md`, then invoke `writing-plans`.

> **Skill:** `superpowers:writing-plans`

Convert the approved design into a bite-sized implementation plan. The plan **must** use the skill's multi-part split — never a single monolithic plan file. All artifacts live under `docs/superpowers/active/YYYY-MM-DD-<feature>/`:

- **Index plan** (`YYYY-MM-DD-<feature>-plan.md`) — the plan document header, the list of part plans, and cross-part dependency information (implementation sequence, shared interfaces/types, how parts depend on each other). It carries no tasks itself.
- **Part plans** (`YYYY-MM-DD-<feature>-plan-<part>.md`, where `<part>` is a sequential number `1`, `2`, …) — split by subsystem, purpose, or phase. Each part holds the bite-sized tasks for that slice and references other parts where dependencies exist.
- **Goal file** (`YYYY-MM-DD-<feature>-goal.md`) — written after the plan, from the skill's `goal-prompt.md` template, with `Persona`, `Context`, `Tasks`, and `Success Criteria` sections filled with concrete values. Keep it shorter than the plan.
- **Manual file** (`YYYY-MM-DD-<feature>-manual.md`) — a developer-facing walkthrough for manually verifying the feature in a live environment (UI flows, edge cases, integration checks). Written for a human, not an agent.
- **Check file** (`YYYY-MM-DD-<feature>-check.md`) — created now, during planning, but **used after implementation**. Authoring it up front locks the acceptance criteria into the plan; it is ticked off in [Step 3](#step-3--verify-test-and-ci) once code lands. It covers: all plan tasks complete, local CI green (`./scripts/ci.sh`), PR open, and all GitHub Actions workflows green. This is a deliberate project convention — the `writing-plans` skill defers check-file creation to post-implementation; we author it during planning instead.

Every task in the part plans must include: files to touch, TDD steps, verification commands, and a commit step. Follow the parallelism rule below when structuring tasks.

**Parallelism:**

- **Plan-level:** Explicitly identify which tasks can run in parallel and which must remain sequential. Tasks are independent when they touch different files and share no produced interfaces or types. Dependent tasks run sequentially. If one parallel task fails and another succeeds: keep the successful commit, fix the failing task independently, then re-run its review loop. Do not roll back the passing task.
- **Task-level:** Within a task's loop, `spec-reviewer` and `code-quality-reviewer` run in parallel — both are read-only. `implementer` and `tester` are always sequential — tester consumes the implementer's output.

### Step 1 — Set Up Worktree (If not created)

> **Skill:** `superpowers:using-git-worktrees`

Create an isolated git worktree for the work. Follow the Git and worktree standards in [`docs/specs/git.md`](./git.md).

### Step 2 — Execute Tasks via Subagents

> **Skill:** `superpowers:subagent-driven-development`

For each task in the plan, dispatch a fresh subagent with precisely crafted context (no session history bleed). Use the workspace subagents: `implementer`, `spec-reviewer`,`code-quality-reviewer` and `tester`. Each subagent must choose the relevant standards to follow based on the task. Default standards by role:

| Role | Default standards |
|---|---|
| `implementer` | [`testing.md`](./testing.md), [`coding.md`](./coding.md), and [`git.md`](./git.md) |
| `tester` | [`testing.md`](./testing.md) |
| `code-quality-reviewer` | [`coding.md`](./coding.md) |
| `spec-reviewer` | The approved design, implementation plan, and acceptance criteria |
| `consultant` | Escalation-only: use for difficult or severe blockers/problems when the current agent is stuck, unsure, or not confident; do not overuse |

Each task goes through an implement plus parallel review loop with bounded convergence:

1. **Implement** — `implementer` follows TDD (failing test first, minimal production code, refactor) and the relevant testing, coding, and git standards.
2. **Test** — `tester` writes, runs and verifies test coverage for the implementation using the testing standard. If gaps or failures found, `implementer` fixes and `tester` re-verifies.
3. **Spec review + Quality review (parallel)** — dispatch `spec-reviewer` and `code-quality-reviewer` simultaneously against the finished implementation. `code-quality-reviewer` uses the coding standard. Both are read-only and always safe to run in parallel.
   - If either reviewer finds **Critical or Important** issues, `implementer` fixes all issues from both reviewers in one pass, then both reviewers re-run in parallel with context carryover (prior findings, what was fixed, intentional decisions).
   - If reviewers find **only Minor** issues, they return "Approved with notes." No re-review triggered — the task is complete. Fix trivial Minor items inline; defer non-trivial ones to the final review.
4. **Convergence safeguards** — the review loop is bounded:
   - **Max 2 iterations** per task. After 2 cycles, escalate remaining issues to the coordinator.
   - **Stall detection** — if iteration N has the same findings as N-1, stop and escalate.
   - **Progress requirement** — each iteration must have strictly fewer Critical + Important findings than the previous. If the count stays flat or increases, escalate.
5. Mark task complete only when both reviewers approve (or coordinator accepts after escalation).
6. Commit the task work.

**Subagent context (every dispatch):** Every subagent starts cold — no knowledge of prior messages or tool outputs. Each dispatch must include: worktree path · full task text · relevant spec excerpts · any types/interfaces from prior tasks · verification command + expected output. No session history. No unrelated tasks. No full file dumps unless the task requires it.

### Step 3 — Verify, Test, and CI

> **Skill:** `superpowers:subagent-driven-development`

Once all tasks are done, dispatch three subagents in sequence — each runs a fix-hard loop internally before the next begins.

1. **Checklist subagent** — reads the check file (`YYYY-MM-DD-<feature>-check.md`) and verifies every item under `## Review`, `## Decisions`, and `## Risks` is satisfied by the implementation. Fix any gaps and re-verify until the checklist is fully green.

2. **CI subagent** — runs `./scripts/ci.sh`. If any job fails: reads the log, fixes locally, reruns either `./scripts/ci.sh <job ...>` or `./scripts/ci.sh --from <job>` as appropriate during the fix loop, then reruns the full `./scripts/ci.sh` before PR creation.

### Step 4 — Open a PR and Verify CI

> **Skill:** `superpowers:finishing-a-development-branch`

```bash
git push -u origin <branch-name>
gh pr create ...
```

- If the PR branch has conflicts with `master`, fetch latest refs and resolve them on the feature branch before proceeding:

```bash
git fetch origin
git merge origin/master
```

- Resolve conflicts locally in the worktree, rerun the required verification commands, and push the updated branch.
- Monitor the GitHub Actions CI run.
- If any job fails: read the log, fix locally, push, repeat.
- **Do not proceed until every CI job is green.**

### Step 5 — Report to User

Send a completion report that includes:

- What was implemented (task-by-task summary).
- PR link and CI status (all green).
- Any notable decisions, trade-offs, or deferred items.

### Step 6 — Update Progress and Archive Docs

After the PR is merged and CI is green:

1. Update `docs/specs/progress.md` — add a summary of the delivered work under the appropriate section.
2. Archive completed Superpowers docs:
   - Move finished feature folders from `docs/superpowers/active/YYYY-MM-DD-<feature>/` → `docs/superpowers/done/YYYY-MM-DD-<feature>/`.
3. Commit the housekeeping directly to `master` (or include it in the PR before merge).

## Coding Standards

Alfred follows standards in [`docs/specs/coding.md`](./coding.md#code-documentation).

## Git Standards

Use [`docs/specs/git.md`](./git.md) for branch, commit, and worktree standards.
