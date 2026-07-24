# Contribution Guide

## PR Process

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/your-feature`
3. Make changes following coding standards
4. Run `cargo check` — must pass with 0 errors
5. Verify manually (no test harness)
6. Submit PR with description of changes

## Coding Standards

- Follow existing code style (see `DeveloperGuide.md`)
- No `unwrap()` / `expect()` in production code
- `// SAFETY:` comment on every `unsafe` block
- `pub(crate)` visibility by default
- Reuse allocations in per-frame code paths
- No `dyn` trait objects
- No external crate dependencies (libsarga is the sole dependency)

## Testing Approach

ADE currently has no test harness (`#![no_std]`, `#![no_main]`, no `#[test]`).

Verification methods:
- **Manual boot-time testing**: Flash image, boot in QEMU, test features
- **Build verification**: `cargo check` with `#![deny(warnings)]`
- **Automation**: CI runs `cargo check` on every PR

When adding tests:
- Prefer `#[cfg(test)]` modules with inline asserts
- Use `#![cfg(test)]` for test-only imports
- Test at the module level (unit tests), not integration tests

## Documentation Requirements

- Public API functions: doc comment `/// Description`
- Module-level: `//! Module description` at file top
- `unsafe` blocks: `// SAFETY:` comment explaining invariants
- Non-obvious design decisions: comment explaining rationale
- Constants: document units and valid range

## Commit Message Format

```
<scope>: <description>

<optional body>
```

Examples:
```
wm: fix window closing leaves dangling focus
ipc: add channel subscribe/unsubscribe API
docs: add compositor architecture documentation
```

Scopes: `wm`, `ipc`, `render`, `service`, `desktop_api`, `a11y`, `portal`, `launcher`, `settings`, `theme`, `docs`, `ci`, `release`
