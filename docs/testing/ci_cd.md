# CI/CD Pipeline

SkyOS uses GitHub Actions for CI. The pipeline runs on every push and pull request to `main`.

## CI Workflow (`.github/workflows/ci.yml`)

1. **fmt** — `cargo fmt --check` (nightly)
2. **clippy** — `cargo clippy -Zbuild-std=core,alloc --target x86_64-sarga.json -- -D warnings`
3. **check-all-targets** — debug + release build of the userspace workspace
   (`cargo build -Zbuild-std=core,alloc --target x86_64-sarga.json [--release]`)
4. **integration-qemu** — full end-to-end boot test:
   - Checks out the kernel repo (`SKYIOUS/SKYIOUS-KERNEL`) beside this repo
   - Builds the kernel with `--features net,smp,ai_rule,self_test`
   - Builds userspace release, runs `build_initrd.py`, builds the UEFI bootimage
     (`kernel/builder`), and creates an ISO with `scripts/make_iso.py`
   - Boots the ISO in QEMU (OVMF, 512M, 2 cpus, e1000 NIC) with a 120s timeout
   - Asserts the log contains a `login:` prompt; fails on any `not ok` (TAP) selftest
     output; reports the kernel selftest TAP summary when present

The kernel's `self_test` feature emits TAP-format results (`TAP version 13`, `ok`/`not ok`) that
the CI log check validates.

## Build Artifacts

Successful builds produce:
- Userspace binaries under `target/x86_64-sarga/release/`
- Kernel ELF `kernel/kernel/target/x86_64-unknown-none/release/vahi_kernel`
- UEFI boot image `kernel/target/x86_64-vahi/release/bootimage-vahi_kernel.bin`
- Initrd `initrd.tar`
- ISO `release/skyos-<version>.iso`

## Release Process

1. Version bump following semver
2. Changelog update (`docs/CHANGELOG.md`)
3. Tagged release on GitHub
4. Boot-image/ISO publication via GitHub Releases

## Local Verification

```bash
make qemu-test    # build ISO, boot in QEMU, assert login prompt
```
