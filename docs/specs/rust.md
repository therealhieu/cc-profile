# Rust Standards

## Table of Contents

- [Principles](#design-principles)
- [Rust Development Standards](#rust-development-standards)
  - [Formatting](#formatting)
  - [Code documentation](#code-documentation)
  - [Linting](#linting)
  - [Naming and module structure](#naming-and-module-structure)
  - [Public APIs](#public-apis)
  - [Ownership and borrowing](#ownership-and-borrowing)
  - [Error handling](#error-handling)
  - [Pattern matching and control flow](#pattern-matching-and-control-flow)
  - [Generics, traits, and macros](#generics-traits-and-macros)
  - [Unsafe code](#unsafe-code)
  - [Cargo, features, and dependencies](#cargo-features-and-dependencies)
  - [Concurrency and async](#concurrency-and-async)
  - [Security and boundary handling](#security-and-boundary-handling)
  - [Review checklist](#review-checklist)

# Principles

Code should be correct first, consistent with its surroundings, simple to understand, reliable, maintainable under change, and extensible only from demonstrated needs.

- **Correctness.** Encode invariants in types, make state transitions explicit, handle failures precisely, and validate external input at boundaries before trusting data internally.
- **Reliability.** Focus on testing. Write fast, deterministic tests—including unit, integration, and documentation tests—to verify success paths, error handling, and edge cases, ensuring code remains correct and regression-free.
- **Consistency.** Follow established project and Rust ecosystem patterns for naming, module structure, error handling, tests, and documentation so similar problems have similar solutions.
- **Simplicity.** Prefer direct code, clear ownership, explicit control flow, and narrow APIs over abstraction until real repetition proves the abstraction clearer.
- **Maintainability.** Keep modules cohesive, dependency direction obvious, public APIs small, and changes focused on one problem. Delete unused code and avoid unrelated refactors.
- **Extensibility.** Design extension from real requirements, not speculation. Avoid catch-all modules, hidden globals, cycles, feature shims, and extension points without current callers.

## Rust Development Standards

### Formatting

Rust code follows the default Rust style.

- Run `cargo fmt` before committing Rust changes.
- Keep rustfmt configuration minimal and stable; do not customize formatting for personal preference.
- If the project uses a Rust edition with style-edition differences, set `style_edition` in `rustfmt.toml` so editor formatting and CI formatting match.
- Do not hand-format code that rustfmt owns.

### Code documentation

Rust code documentation follows rustdoc and the Rust API Guidelines.

- Use `//!` for crate and module docs; use `///` for public item docs.
- Document every public item that is part of the crate contract. Use this structure when sections apply:

  ````text
  One-line summary sentence.

  Optional details explaining behavior, invariants, ownership expectations, edge cases, or domain meaning.

  # Parameters

  - `name`: Meaning, units, accepted values, constraints, or interactions.

  # Returns

  A [`ReturnType`] describing the successful return value.

  # Errors

  Returns [`ErrorType`] when the operation fails for documented conditions.

  # Panics

  Panics when documented caller-reachable conditions occur.

  # Safety

  The caller must uphold every invariant required for soundness.

  # Examples

  Optional; include only when the example clarifies usage, invariants, or non-obvious behavior.

  Short sentence describing what the example demonstrates.

  ```rust
  # // Hidden setup when needed.
  let value = example_call();
  assert_eq!(value, expected);
  ```
  ````

- Do not restate types already visible in the signature unless the type's semantic meaning helps callers understand the contract.
- Add `# Parameters` only when callers need information not visible from parameter names and types. Document one parameter per bullet.
- Add `# Returns` when the return value's semantic meaning is not obvious from the signature. For `Result<T, E>`, describe the `Ok(T)` value in `# Returns` and failure conditions in `# Errors`.
- Add `# Examples` only when an example clarifies public usage, important invariants, or non-obvious behavior. Do not require examples for every public function, method, struct, enum, trait, macro, or type alias. Link to an existing example instead of duplicating one.
- Format `# Examples` with a short sentence describing what the example demonstrates, followed by a fenced `rust` code block.
- Prefer tested Rust examples when examples are included; verify them with `cargo test --doc --workspace`. Use hidden `#` setup lines to keep examples complete but readable, use `?` for fallible examples, use `no_run` only when the example should compile but not execute, use `ignore` only when no testable form is practical, and use `compile_fail` for invalid-usage examples.
- Add `# Errors` when returning `Result` and callers need to understand failure conditions.
- Add `# Panics` for reachable panic conditions.
- Add `# Safety` for unsafe functions, unsafe traits, and unsafe trait methods; state every invariant the caller must uphold.
- Use intra-doc links for related types, traits, methods, modules, and concepts.
- Keep generated rustdoc free of unhelpful implementation details; prefer private items, `pub(crate)`, or narrowly scoped `#[doc(hidden)]` glue.
- For private code, prefer clear names and types over comments. Add a short comment only for non-obvious invariants, unsafe justifications, protocol constraints, or external compatibility requirements.

### Linting

Clippy is the baseline for idiomatic Rust feedback.

- Run `cargo clippy --all-targets --all-features -- -D warnings` when the crate supports all-feature checks.
- Treat default Clippy groups as actionable: correctness, suspicious, style, complexity, and perf.
- Do not enable all of `clippy::restriction`, `clippy::nursery`, or `clippy::pedantic`; cherry-pick individual lints only when they match project needs.
- Use `#[allow(...)]` narrowly, closest to the lint, with a short reason when the exception is not obvious.

### Naming and module structure

Use Rust ecosystem naming conventions.

| Item | Convention |
|---|---|
| Crates, modules, functions, methods, variables | `snake_case` |
| Types, traits, enum variants | `UpperCamelCase` |
| Constants and statics | `SCREAMING_SNAKE_CASE` |
| Lifetimes | Short lowercase names, usually `'a` |

- Keep modules cohesive around domain concepts, not technical buckets.
- Keep `lib.rs` and `main.rs` focused on crate wiring and public surface area.
- Re-export intentionally; a re-export is part of the API.
- Avoid wildcard imports outside tests and tightly scoped prelude-style modules.

### Public APIs

Public APIs should be hard to misuse and easy to evolve.

- Keep fields private unless direct field access is the intended contract.
- Prefer constructors and methods that preserve invariants.
- Use `From`, `TryFrom`, `AsRef`, `AsMut`, `Borrow`, and iterator traits instead of ad-hoc conversion names.
- Use `iter`, `iter_mut`, and `into_iter` for collection traversal methods.
- Derive common traits when they are semantically correct: `Debug`, `Clone`, `Copy`, `Eq`, `PartialEq`, `Ord`, `PartialOrd`, `Hash`, `Default`.
- Implement `Display` for user-facing formatting and keep `Debug` useful for diagnostics.
- Use builders when construction has many optional fields or order-independent configuration.

### Ownership and borrowing

Prefer APIs that make ownership transfer explicit.

- Accept borrowed values when the function does not need ownership: `&str`, `&[T]`, `&Path`, or trait bounds such as `AsRef<Path>` where useful.
- Take ownership when storing, spawning, or transferring responsibility.
- Avoid cloning to satisfy the borrow checker until the ownership model is understood.
- Return iterators when callers can consume a sequence lazily; return collections when materialization is part of the contract.

### Error handling

Rust code should distinguish recoverable failures from programmer errors.

- Return `Result<T, E>` for recoverable failures.
- Use `panic!`, `expect`, and `unwrap` only when failure means a violated invariant, a test setup failure, or an unrecoverable startup/configuration error with clear context.
- Library code should expose meaningful error types that implement `std::error::Error`.
- Application code should add context at boundaries where the operation being attempted is known.
- Do not erase structured errors into strings before callers have a chance to inspect them.

### Pattern matching and control flow

Make state handling exhaustive and visible.

- Prefer `match` when handling all variants matters.
- Prefer `if let` or `let ... else` for a single interesting branch.
- Keep early returns for validation and error paths; avoid deeply nested success paths.
- Do not use `_` in public-contract matches when new variants should force review.

### Generics, traits, and macros

Use abstraction when it reduces coupling for current code.

- Prefer concrete types until multiple real callers need abstraction.
- Keep trait contracts small and behavior-focused.
- Do not create traits solely to mock dependencies; prefer testing through public behavior.
- Use generics when callers benefit from flexibility without making errors or types harder to read.
- Use macros only when functions, traits, or derives cannot express the pattern clearly.

### Unsafe code

Unsafe code is exceptional.

- Do not introduce `unsafe` unless there is no safe Rust alternative that meets the requirement.
- Keep unsafe blocks minimal and isolated behind safe APIs.
- Document each unsafe block with the invariant that makes it sound.
- Add tests that exercise the safe API around unsafe behavior.

### Cargo, features, and dependencies

Cargo configuration is part of the code contract.

- Keep feature flags additive; enabling a feature should not silently disable behavior.
- Do not change the minimum supported Rust version unless the task requires it.
- Prefer standard library functionality before adding a crate.
- Add third-party crates only when they are maintained, appropriately licensed, and reduce project complexity.
- Keep dependency configuration explicit; avoid broad default features when only a small feature set is needed.

### Concurrency and async

Use concurrency only where it simplifies ownership of work or is required for performance.

- Prefer message passing or owned task state over shared mutable state.
- Use `Arc` when ownership is shared across tasks or threads; add `Mutex`/`RwLock` only for data that must mutate across owners.
- Do not hold blocking locks across `.await` points.
- Keep cancellation and shutdown paths explicit for long-running tasks.

### Security and boundary handling

Treat external input as untrusted.

- Validate and parse input at boundaries before passing typed data inward.
- Avoid shelling out with interpolated user input; pass arguments as structured command arguments.
- Do not log secrets, tokens, credentials, or raw sensitive payloads.
- Prefer safe standard-library and well-reviewed crate APIs for path, URL, serialization, and cryptographic operations.

### Review checklist

Before marking Rust work complete:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --workspace
cargo test --doc --workspace
```

If a command does not apply to the crate, record the reason in the task or PR notes.
