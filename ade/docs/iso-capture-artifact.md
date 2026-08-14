# ISO Capture Artifact — known-good kernel regression target

**Date:** Aug 11, 2026 · **Method:** read-only source inspection (no Rust changed)

## Goal

While the kernel's major change is in flight (remote `master` no longer boots to
userspace), capture the *working* ISO — `release/skyos-0.6.0.iso` (Aug 9 19:28,
8.4 MB) — as a reproducible CI artifact: build at a known-good kernel commit,
boot it, and save the ISO + serial log so the integration harnesses have a
stable target to diff against.

## Provenance of the working ISO (evidence)

| Fact | Evidence |
|---|---|
| ISO built Aug 9 19:28 from the **local** kernel tree | `release/skyos-0.6.0.iso` mtime; `git status --short` in `SKYIOUS-KERNEL` shows **50 uncommitted files** on top of `a18848f` |
| Kernel source commit | local HEAD `a18848f` (Aug 5) — **18 commits ahead** of kernel remote `master` (`c703285`, Jul 24); `git log a18848f..origin/master` is empty ⇒ remote is fully contained in local |
| Never committed | the boot-relevant fixes live in the 50-file dirty delta, not in any commit |
| Feature set | kernel default features (`kernel/Cargo.toml:36` = `smp, net, ai_rule, ext4`), no `self_test` — matches the boot log's "~590 KB kernel, no self_test" |
| Boots to userspace | the console getty path reaches `login:`; the GUI (login-manager) hangs at window creation (the `[login] failed to create window` respawn loop) |

## The blocker: CI cannot reproduce this ISO today

1. **No known-good *commit* exists on any remote.** The working state is
   `a18848f` + 50 dirty files. `a18848f` itself was never pushed, and even it
   would **not** reproduce the ISO (the dirty files are part of what boots).
2. **The only available credential is read-only on both repos.** `gh api`
   reports `push: false` on both `SKYIOUS/SkyOS` and `SKYIOUS/SKYIOUS-KERNEL`,
   so neither the kernel fix commit nor a marker ref can be pushed from here.
3. **No remote commit is a safe stand-in.** The last kernel remote commit with
   a successful CI build was `5128b003` (June 19) — it predates the current
   userspace ABI (the ~60-syscall surface, packed-key input), so building the
   *current* userspace against it would not boot. Pinning to it would produce a
   misleadingly "captured" artifact.

## The workflow change (this session)

`.github/workflows/release-iso.yml` (the existing ISO build workflow) now:

- **`kernel_ref` input** (default `master`) threaded into the kernel checkout
  `ref:` — the operator passes the pushed known-good commit once it exists.
- **`boot_capture` input** (seconds; `0` skips) — boots the ISO headless
  (`-display none -monitor none -serial file:capture_serial.log`) and uploads
  the serial log as a second artifact alongside the ISO.
- **Reachability gate** — fails loudly if `login:` never appears in the serial
  log, so a non-booting pin cannot silently produce a "captured" artifact:
  the job reports `FAIL: kernel_ref=<ref> did not reach userspace` with the
  log tail. This is the honest-consequence design: firing it at `master`
  *today* fails at the gate, which is correct.

The build path is unchanged from the existing workflow (default-feature kernel
build = the working ISO's no-`self_test` config, builder → `make_iso.py`).

## Unblock steps (needs a write-capable credential)

1. In `SKYIOUS/SKYIOUS-KERNEL`: commit the 50-file dirty tree (or at least the
   boot-critical subset), then push `a18848f`'s 18 commits + the fix commit.
2. Fire `Build Release ISO` → `workflow_dispatch` with
   `kernel_ref=<that commit>` and a boot `boot_capture` (e.g. 90).
3. The gate asserts `login:` in `capture_serial.log`; the ISO + serial log are
   downloadable artifacts.
4. Optionally, temporarily pin the `integration` job's kernel checkout
   (`ci.yml`, `Checkout kernel` step) to that ref until `master` is fixed, so
   the harnesses run against the known-good target rather than failing at
   build/boot.

## Verified locally (Aug 14, 2026)

Re-ran the workflow's own reachability gate (`tests/probe_iso_boot.py`, which
mirrors the `boot_capture` QEMU command exactly: `-bios OVMF.fd -cdrom … -m
512M -smp 2 -display none -monitor none -serial file:`) against every ISO in
`release/`:

| ISO | Built | Size | Reaches `login:`? |
|---|---|---|---|
| `skyos-probe.iso` | Aug 9 19:14 | 8.4 MB | **PASS** — `[init] SARGA init starting`, `starting service:`, `login:` |
| `skyos-kgtest.iso` | Aug 12 | 8.4 MB | FAIL — hangs at `[APIC] init_timer...` |
| `skyos-0.6.0-itg2.iso` | Aug 12 | 8.4 MB | FAIL — same APIC hang |
| `skyos-0.6.0.iso` | Aug 12 | 30 MB | FAIL — same APIC hang |
| `skyos-focuscheck-1639.iso` | Aug 14 16:39 | 9.5 MB | FAIL — same APIC hang |

So the structural-validity loop is now closed with boot evidence: the pinned
builder's hybrid contract (MBR+GPT+El Torito, `test_make_iso_pycdlib.py`) is
byte-valid **and** the surviving known-good image actually boots to the getty
under OVMF. The later ISOs fail at `[APIC] init_timer...` — the same kernel
breakage the iso-capture doc's provenance section predicts (remote `master` no
longer reaches userspace; the working state was the local Aug 9 tree). The
`boot_capture` gate in `release-iso.yml` would FAIL on any of the later ISOs,
which is the intended honest behavior.

## Related

- `ade/docs/session-lifecycle.md` — respawn accounting and logout contract the
  captured serial log should satisfy.
- `ade/docs/kernel-owns-facility-audit.md` — userspace↔kernel ownership map.
- `ade/docs/kernel-gui-window-fix.md` — the GUI window-creation loop being
  fixed by the kernel rewrite.
