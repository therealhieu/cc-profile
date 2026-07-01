# cc-profile

Profile management for Claude Code endpoints and models.

## Install

```bash
cargo install cc-profile
```

## Update

Self-update support is planned; use `cargo install cc-profile --force` until the built-in update command ships.

## Development and testing

- **`CC_PROFILE_CLAUDE_BIN`** — Overrides the binary launched by `cc-profile start`. Intended for integration tests and manual debugging; defaults to `claude` when unset.
- **Test Claude shim** — Source lives at `tests/fixtures/cc-profile-test-claude.rs`. Integration tests compile it on demand; it is not installed with `cargo install`. The shim requires **`CC_PROFILE_TEST_CLAUDE_OUTPUT`** to point to a writable file; without it, the shim exits with an error.