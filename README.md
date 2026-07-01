# cc-profile

Profile management for Claude Code endpoints and models.

## Install

```bash
cargo install cc-profile
```

## Update

Self-update support is planned; use `cargo install cc-profile --force` until the built-in update command ships.

## Development and testing

Run the same checks as GitHub Actions locally:

```bash
./scripts/ci.sh
```

Individual jobs: `fmt`, `clippy`, `test`, `package`, `publish-dry-run`. Use `./scripts/ci.sh --help` for details.

- **`CC_PROFILE_CLAUDE_BIN`** — Production `cc-profile start` reads this variable and launches that executable instead of `claude` when set. Use only for trusted test or debug binaries (for example the integration-test shim); unset it after debugging so launches go back to the real Claude Code CLI.
- **Test Claude shim** — Source lives at `tests/fixtures/cc-profile-test-claude.rs`. Integration tests compile it on demand; it is not installed with `cargo install`. The shim requires **`CC_PROFILE_TEST_CLAUDE_OUTPUT`** to point to a writable file; without it, the shim exits with an error.