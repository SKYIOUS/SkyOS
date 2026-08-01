# Release Checklist

Use this checklist when preparing a new SkyOS release.

## Pre-Release

- [ ] **All CI passes** — `cargo fmt --check`, `clippy -D warnings`, full build, unit tests, QEMU smoke test
- [ ] **Changelog updated** — see `docs/CHANGELOG.md`; ensure all user-facing changes are listed
- [ ] **Version bumped** — update `Cargo.toml` workspace version and any hardcoded version strings
- [ ] **`docs/CHANGELOG.md`** has a new `[Unreleased]` section for the next cycle

## Build Verification

- [ ] **Userspace builds cleanly** — `cargo build -Zbuild-std=core,alloc --target x86_64-sarga.json --release`
- [ ] **Kernel builds cleanly** — `cargo build --release -Zbuild-std=core,alloc --target x86_64-unknown-none --features net,smp,ai_rule,ext4,self_test`
- [ ] **Initrd created successfully** — `python3 build_initrd.py`
- [ ] **Bootimage created** — builder runs without errors
- [ ] **ISO created** — `python3 scripts/make_iso.py <version>` produces valid ISO

## Boot Tests

- [ ] **QEMU boot (nographic)** — system reaches login prompt within 60s
- [ ] **QEMU boot (graphical)** — GUI desktop loads, window manager functional
- [ ] **Serial console works** — keyboard input and shell interaction responsive
- [ ] **Networking** — DHCP acquires address, `ping 8.8.8.8` succeeds, `curl` fetches HTTP
- [ ] **Filesystem** — root filesystem mounts, file create/read/write/delete works
- [ ] **Init system** — services spawn, respawn on crash
- [ ] **Test binaries pass** — run `sigchld_test`, `sigint_test`, `perm_test`, `futex_test` in QEMU

## Integration Tests

- [ ] **`tests/qemu_boot.sh`** passes on a fresh clone
- [ ] **`tests/test_boot.ps1`** passes on Windows
- [ ] **`tests/test_login.ps1`** passes (root/root login)
- [ ] **All CI workflows pass** on the release branch

## Release Artifacts

- [ ] **Release ISO** uploaded to GitHub Releases (`skyos-<version>.iso`)
- [ ] **Source archive** (or tag) included
- [ ] **Checksums** computed (`sha256sum skyos-*.iso > checksums.txt`)
- [ ] **Release notes** drafted with notable changes

## Post-Release

- [ ] Tag created: `git tag v<version> && git push --tags`
- [ ] Git branch merged to `main` if release was from a release branch
- [ ] Announcement drafted (if applicable)

## Borrowed Algorithms & Citations

SkyOS builds on decades of OS research. Key sources:

| Component | Reference |
|-----------|-----------|
| Higher-half kernel mapping | JamesM's kernel development tutorials, OSDev wiki |
| Buddy allocator | Knuth (Art of Computer Programming, Vol. 1) |
| Slab allocator | Bonwick (Solaris kernel, USENIX 1994) |
| Ext2 filesystem | Rémy Card (1993), Linux fs/ext2 implementation |
| FAT32 | Microsoft FAT specification, ECMA-107 |
| IPv4/IPv6 stack | smoltcp library by whitequark, RFC 791/8200 |
| TCP congestion control | RFC 5681 (Reno), smoltcp implementation |
| Scheduler (multi-level priority RR) | McKusick et al. (4.4BSD Design), Linux CFS inspiration |
| ELF loading | System V ABI specification |
| UEFI boot | bootloader crate by Philipp Oppermann, UEFI 2.9 spec |
| System V ABI (x86_64) | AMD64 APM, System V psABI |
