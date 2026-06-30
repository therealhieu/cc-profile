# Git Standards

## Worktrees

Create an isolated git worktree for feature work:

```bash
git worktree add .worktrees/<branch-name> -b <branch-name>
```

- Name must follow Git worktree rules below: no agent prefix, flat path.
- **No agent prefix** — names must NOT include agent identifiers (`codex-`, `claude-`, `gpt-`, etc.).
- **Flat paths only** — single segment under `.worktrees/`. No nested subdirectories.
  - Good: `.worktrees/impl`, `.worktrees/feat-auth`, `.worktrees/fix-cache`
  - Bad: `.worktrees/feat/implement`, `.worktrees/claude/fix`
- All implementation work happens inside this worktree.

## Commit Messages

Use Conventional Commits with concise, imperative summaries.

```text
<type>(optional-scope): <imperative summary>
```

### Types

| Type | Use for |
|---|---|
| `feat` | New user-facing capability |
| `fix` | Bug fix |
| `docs` | Documentation-only change |
| `test` | Test-only change |
| `refactor` | Code restructuring without behavior change |
| `chore` | Repo maintenance, tooling, or configuration |
| `ci` | CI workflow changes |
| `build` | Build system, package, or dependency changes |

### Rules

- Keep the subject under 72 characters.
- Use lowercase types and scopes.
- Do not end the subject with punctuation.
- Use the summary to describe the outcome, not the implementation detail.
- Keep each commit focused on one logical change.
- Add a body only when the change needs context that is not obvious from the diff.

### Good examples

```text
feat(protocol): add session contract mocks
fix(client): abort SSE retry after shutdown
docs(git): document commit conventions
test(protocol): cover mock event payloads
chore(repo): update Biome configuration
ci(repo): run coverage checks on pull requests
build(web): add Vite test dependency
```

These are good because the type is specific, the scope is useful, and the summary describes the intent.

### Bad examples

```text
fixed stuff
WIP
changes
update files
feat: Added a bunch of tests and fixed some bugs.
chore: misc
```

These are bad because they are vague, too broad, non-imperative, or include unnecessary punctuation.

## Commit identity

Always commit using the repository's configured git identity (`git config user.name` / `user.email`).

- Never pass `--author` to `git commit`.
- Never pass `-c user.name=` or `-c user.email=` on commit.
- Never set `GIT_AUTHOR_*` or `GIT_COMMITTER_*` environment variables.
- Do not amend or rebase to rewrite an author unless the user explicitly asks.

If the configured identity is missing, stop and ask the user instead of inventing one.

### Commit bodies

Add a body when the subject alone does not explain the decision:

```text
fix(client): abort SSE retry after shutdown

Abort pending retry timers when the stream closes so tests and clients do not
observe events after shutdown.
```

Avoid bodies that restate the diff:

```text
test(client): update SSE tests

Changed the SSE test file and edited the retry test.
```
