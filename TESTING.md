# SkyOS Testing Guide

## Overview

SkyOS uses a multi-tiered testing strategy due to its bare-metal target architecture. This document explains the current testing infrastructure and gaps.

## Test Infrastructure

### Kernel Self-Tests (Primary Path)

The kernel includes built-in self-tests that run at boot time. These are the primary verification mechanism for kernel functionality.

- **Location:** Kernel crate (separate repository)
- **Trigger:** Boot with appropriate flags or feature `self_test`
- **Output:** TAP (Test Anything Protocol) format via serial console
- **CI Integration:** `.github/workflows/ci.yml` checks for TAP output and `not ok` failures

### Host-Side Test Frameworks

Two test crates exist for host-side testing but are **excluded from the workspace**:

- **`tests/skyos-test-core`** - Test framework library (serde-based)
- **`tests/skyos-test`** - CLI tool for running tests and generating reports

These are designed for testing logic that can run on the host machine, not on the bare-metal target.

### CI Testing

Current CI (`.github/workflows/ci.yml`) includes:
- `fmt` - Code formatting check
- `clippy` - Lint checks
- `build` - Compilation verification
- `integration-qemu` - Boot test and kernel self-test verification

**Not included:** `cargo test` (unit tests)

## Testing Gap: No CI-Wired Unit Tests

### Current State

- **Host-runnable `#[cfg(test)]` modules now exist** in `libsarga` (`errno.rs`, `net.rs`, `semver.rs`, `serialize.rs`) and `ade` (`sys/{audio,display,input,network}.rs`, `util/{automation,developer,extension,plugin,sdk}.rs`) with pure-logic `#[test]` functions.
- **They are not run in CI.** `.github/workflows/ci.yml` runs `fmt`, `clippy`, and cross-target builds only — no `cargo test` step.
- Test crates (`skyos-test`, `skyos-test-core`) are excluded from `Cargo.toml` workspace members.

### Why These Tests Are Not Run in CI

1. **Bare-Metal Target:** The userspace target `x86_64-sarga` is a bare-metal target. Most crates use `#![no_std]` (e.g. `libsarga` is `#![no_std]` + `#![feature(alloc_error_handler)]`) and cannot use the standard Rust test harness when built for that target.

2. **No Test Harness:** The kernel has `panic = "abort"` and no test harness configured. The `#![no_std]` + `#![no_main]` pattern prevents standard `cargo test` from working on the bare-metal target.

3. **Platform-Specific Code:** Much of the code (syscalls, hardware access) requires the actual kernel environment to function correctly.

### Why Test Crates Are Excluded

The `tests/skyos-test` and `tests/skyos-test-core` crates are excluded from the workspace because:

- They are **host-side tools** designed to run on the development machine, not on SkyOS
- They depend on `serde` and other standard library features not available in the bare-metal target
- Including them would complicate the build for the bare-metal target

## Remediation Options

### Option 1: Add Host-Side Unit Tests (Recommended for Pure Logic)

For pure logic that doesn't depend on syscalls or hardware:

1. Add `#[cfg(test)]` modules with `#[test]` functions
2. Use `#[cfg(target_os = "linux")]` or similar to only run tests on host
3. Add `cargo test` step to CI for host-only tests

**Example:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_decode() {
        assert_eq!(hex_decode(b"48656c6c6f"), Some(b"Hello".to_vec()));
    }
}
```

### Option 2: Enable Test Crates in Workspace

If `skyos-test` and `skyos-test-core` are meant to be actively used:

1. Add them to `Cargo.toml` workspace members
2. Mark them with `[[bin]]` configuration if needed
3. Add build/test steps to CI

### Option 3: Continue with Kernel Self-Tests (Current Path)

Accept that kernel self-tests are the primary verification mechanism and document this as the intended testing strategy.

## Current Recommendation

**Wire the existing host-runnable tests into CI, and keep adding `#[cfg(test)]` modules for pure logic.**

The existing `#[cfg(test)]` modules in `libsarga` and `ade` cover pure logic (parsing, errno, semver, serialization) that does not need the kernel runtime. A `cargo test --target <host>` step would run them on the CI host. New pure-logic code should follow the same pattern; code that touches syscalls/hardware stays covered by kernel self-tests.

## Adding Unit Tests for Pure Logic

To add unit tests for pure logic (e.g., `libsarga/src/hash.rs::hex_decode`):

1. Add test module with `#[cfg(test)]` guard
2. Use `#[cfg(target_arch = "x86_64")]` or similar to restrict to host
3. Run with `cargo test --target x86_64-unknown-linux-gnu` (or host target)

Example (matches the existing `libsarga/src/errno.rs` test module):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_decode_valid() {
        assert_eq!(hex_decode(b"48656c6c6f"), Some(vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]));
    }

    #[test]
    fn test_hex_decode_odd_length() {
        assert_eq!(hex_decode(b"486"), None);
    }
}
```

## Summary

- **Primary test path:** Kernel self-tests (TAP format, verified in CI)
- **Unit tests:** Host-runnable `#[cfg(test)]` modules exist in `libsarga` and `ade` for pure logic, but are not yet wired into CI
- **Host-side test framework:** Exists but excluded from workspace
- **Recommendation:** Wire the existing host-runnable tests into CI and add more for pure logic; rely on kernel self-tests for system verification
