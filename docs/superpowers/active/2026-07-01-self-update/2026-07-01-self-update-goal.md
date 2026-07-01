# Goal — cc-profile Self-Update

## Persona

You are an implementation agent working on `cc-profile`, a Rust CLI for managing Claude Code endpoint/model profiles. Follow the approved design and part plans in this directory. Optimize for simple, safe behavior and strong tests.

## Context

`cc-profile` needs a complete publishing and update story. Users should be able to install through Homebrew, Cargo, or a standalone installer, then run one command:

```bash
cc-profile update
```

Package-manager installs must update through their package managers. Standalone installs must self-replace only after release asset checksum verification and rollback-safe replacement.

## Tasks

Implement the work in this order:

1. Package readiness and metadata.
2. CI, release artifacts, standalone installer, and Homebrew formula template.
3. `cc-profile update --check`, install method detection, and release lookup.
4. Homebrew/Cargo delegation and standalone self-replacement.
5. Passive update notice, final docs, and end-to-end verification.

Use the part plans:

- `2026-07-01-self-update-plan-1.md`
- `2026-07-01-self-update-plan-2.md`
- `2026-07-01-self-update-plan-3.md`
- `2026-07-01-self-update-plan-4.md`
- `2026-07-01-self-update-plan-5.md`

## Success Criteria

- `cc-profile --version` prints the package version.
- `cc-profile update --check` reports current/outdated status without reading profile config.
- `cc-profile update` delegates Homebrew installs to Homebrew.
- `cc-profile update` delegates Cargo installs to Cargo.
- `cc-profile update` self-replaces standalone installs only after checksum verification.
- Failed updates leave the existing binary usable.
- Release workflow publishes GitHub assets and `SHA256SUMS`.
- README documents install, update, uninstall, and troubleshooting paths.
- `cargo publish --dry-run` passes on a clean tree.
