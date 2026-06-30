# cc-profile
Profile Management for Claude Code Endpoints and Models

## Development and testing

- **`CC_PROFILE_CLAUDE_BIN`** — Overrides the binary launched by `cc-profile start`. Intended for integration tests and manual debugging; defaults to `claude` when unset.
- **`cc-profile-test-claude`** — Integration-test shim kept under `src/bin` so default `cargo nextest run --workspace` can exercise the real launch path end-to-end. Normal `cc-profile` flows do not invoke it. The shim requires **`CC_PROFILE_TEST_CLAUDE_OUTPUT`** to point to a writable file; without it, the shim exits with an error.