# Testing Standards

## Core Rule

Tests prove behavior and protect invariants at the narrowest useful boundary.

```text
private/stateful logic → unit test beside implementation
public crate contract  → integration test under crate tests/
public API example     → rustdoc example
a full repo signal     → ./scripts/ci.sh
```

Prefer tests that explain what broke from the failure name and assertion. Do not add tests only to increase coverage.

## Test Placement

| Test type | Location | Use for |
|---|---|---|
| Unit | Same module/file in `#[cfg(test)] mod tests` | Private helpers, state transitions, parser/normalization rules, edge cases, invariants. |
| Integration | `crates/<crate>/tests/integration/<domain>.rs` (one binary per crate via `tests/integration/main.rs`) | Public crate APIs, CLI behavior, server routes, serialization contracts, cross-module flows. |
| Documentation | Rustdoc examples on public items | Public usage examples that should compile and stay accurate. |

Use unit tests near implementation when logic is private, stateful, or easier to diagnose locally. Use integration tests when behavior matters as an external caller would observe it.

Place each integration domain in `crates/<crate>/tests/integration/<domain>.rs` and
declare it as a `mod` from `crates/<crate>/tests/integration/main.rs`, so the crate
produces a single integration binary. Name the domain file `<domain>.rs` (for
example `config.rs`, `cli.rs`); the binary is `integration`, so the old
`_integration.rs` suffix is dropped. Do not add new files directly under `tests/`
— a file there compiles to its own binary, which re-links the whole dependency
graph. See [lessons.md](./lessons.md) for the compile-vs-link rationale.

Example:

```text
crates/<crate>/src/config.rs
└── #[cfg(test)] mod tests
    ├── config_normalization_trims_header_defaults_when_values_have_padding
    └── config_defaults_return_bind_values_when_host_and_port_are_missing

crates/<server-crate>/tests/integration/
├── main.rs                 # the only compiled binary: `mod`s each domain
├── config.rs               # domain module (was config_integration.rs)
└── router.rs               # domain module
```

### External boundaries (backing-service split)

When a crate integrates with multiple external services, split integration
tests by service under one `tests/integration/main.rs` binary (see
[lessons.md](./lessons.md)):

```text
crates/<crate>/tests/integration/
├── main.rs
├── common/           # shared harnesses, fixtures, seed data
├── mock/             # default CI — in-process mocks or wiremock stubs
└── <service>/        # #[ignore] — requires external infra (Docker, cloud)
```

- **mock/** — runs on every PR (`cargo nextest run -p <crate>`).
- **`<service>/`** — each test is `#[ignore]`; run with
  `cargo nextest run -p <crate> --run-ignored ignored-only` when the backing
  service is available.

## Test Quality

- Name tests with `[subject]_[outcome]_when_[condition]`; never use a `test_` prefix.
- Test observable behavior, not implementation details.
- Keep each test focused on one behavior. Multiple assertions are fine when they describe one outcome.
- Cover meaningful success paths and failure modes: empty input, malformed input, missing fields, duplicates, boundary values, and invalid state transitions.
- Assert structured errors when the error type is part of the contract. Assert user-facing text only when the text is the contract.
- Use `rstest` for repeated table-driven cases; do not use it for single-case tests.
- Do not create traits solely to mock dependencies. Prefer real serializers, codecs, filesystems, and local-compatible services when those boundaries are part of the contract.
- Add property or fuzz tests only when example tests cannot cover the meaningful input space.

## Fixtures and Global State

Tests must be deterministic and isolated.

- Use inline data for small fixtures.
- Use `tempfile` or an approved temporary directory helper for filesystem tests.
- Do not write fixed paths in the repository or process working directory.
- Do not depend on test order, wall-clock sleeps, local machine configuration, or external services.
- Avoid mutating process-global environment. If unavoidable, restore it with a guard and serialize affected tests.
- Clean up spawned threads, tasks, subprocesses, sockets, and temporary files before test exit.
- Put manual verification steps in the feature manual file, not in test comments.

## Coverage and CI

Use `cargo-nextest` for unit and integration tests. Use Cargo's built-in runner for doctests because `cargo-nextest` does not run rustdoc examples.

| Purpose | Command |
|---|---|
| Focused unit/integration tests | `cargo nextest run -p <crate> <filter>` |
| Workspace unit/integration tests | `cargo nextest run --workspace` |
| Documentation tests | `cargo test --doc --workspace` |
| Coverage gate | `cargo llvm-cov nextest --workspace --fail-under-lines 90` |
| Full verification | `./scripts/ci.sh` |

The repository requires at least 90% total line coverage. Use coverage reports to find blind spots, not to justify low-value assertions or brittle tests.

Do not use root `cargo test` as the final workspace signal when `default-members` limits the tested crates. Replace broad `cargo test --workspace` checks with:

```text
cargo nextest run --workspace
cargo test --doc --workspace
```

## Review Checklist

Before marking Rust test work complete, run the repository CI script when available:

```text
./scripts/ci.sh
```

The script verifies:

```text
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo nextest run --workspace
cargo test --doc --workspace
cargo llvm-cov clean --workspace
cargo llvm-cov nextest --workspace --fail-under-lines 90
cargo deny check
```

If a command does not apply, record the reason in the task or PR notes.
