#!/usr/bin/env python3
"""Regenerate SkyOS/ade/docs/kernel-mem-lowwater.md (SPEC-ONLY): the exact
kernel/src/memory/buddy.rs + kernel/src/vfs/ctlfs.rs patch that adds the
/ctl/sys/mem/lowwater node (min_free_pages low-water mark on
FragmentationStats), difflib-generated from the LIVE kernel source and
verified with `git apply --check` (same treatment as Option 2/2b in
kernel-gui-window-fix.md). Idempotent: re-running reproduces the identical
doc (verified at the end).
"""
import difflib
import io
import os
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))          # SkyOS/ade/docs
REPO = os.path.dirname(os.path.dirname(HERE))              # SkyOS root
DOC = os.path.join(HERE, "kernel-mem-lowwater.md")
KERNEL_CANDIDATES = [
    os.path.join(os.path.dirname(REPO), "SKYIOUS KERNEL"),
    os.path.join(os.path.dirname(REPO), "SKYIOUS-KERNEL"),
    os.path.join(os.path.dirname(REPO), "SKYIOUS_KERNEL"),
]


def kernel_root():
    env = os.environ.get("SKYOS_KERNEL_DIR")
    cands = ([env] if env else []) + KERNEL_CANDIDATES
    return next((c for c in cands if c and os.path.isfile(
        os.path.join(c, "kernel", "src", "memory", "buddy.rs"))), None)


def read(p):
    return io.open(p, encoding="utf-8", newline="").read()


def normalize(s):
    # CRLF -> LF for diffing (git apply tolerates LF patches vs CRLF trees).
    return "\n".join(s.splitlines()) + ("\n" if s.endswith(("\n", "\r")) else "")


def apply_one(src, old, new, label):
    n = src.count(old)
    assert n == 1, "%s: expected exactly 1 anchor, found %d" % (label, n)
    return src.replace(old, new)


def _format_range_unified(start, stop):
    """Mirror difflib._format_range_unified (single-line hunks omit ',1')."""
    beginning = start + 1
    length = stop - start
    if length == 1:
        return "%d" % beginning
    if not length:
        beginning -= 1
        length = 2
    return "%d,%d" % (beginning, length)


def patch_for(before, after, path):
    """difflib hunks with autojunk=False: difflib.unified_diff's default
    autojunk treats '}' (frequent in the 200+ line buddy.rs) as junk, which
    can misalign identical braces and emit a non-minimal hunk (a moved '}'
    shown as +/-; still apply-correct but confusing to a patch reader). A
    SequenceMatcher with autojunk=False keeps the hunk minimal and
    unambiguous; git apply --check is the arbiter either way."""
    a = normalize(before).splitlines()
    b = normalize(after).splitlines()
    sm = difflib.SequenceMatcher(None, a, b, autojunk=False)
    out = ["--- a/%s" % path, "+++ b/%s" % path]
    for group in sm.get_grouped_opcodes(3):
        first, last = group[0], group[-1]
        out.append("@@ -%s +%s @@" % (
            _format_range_unified(first[1], last[2]),
            _format_range_unified(first[3], last[4])))
        for tag, i1, i2, j1, j2 in group:
            if tag == "equal":
                out.extend(" " + l for l in a[i1:i2])
            elif tag in ("replace", "delete"):
                out.extend("-" + l for l in a[i1:i2])
            if tag in ("replace", "insert"):
                out.extend("+" + l for l in b[j1:j2])
    return "\n".join(out) + "\n"

def transform_buddy(src):
    src = apply_one(src, """/// Fragmentation tracking for buddy allocator
#[allow(dead_code)]
struct FragmentationStats {
    total_free: usize,
    fragmented_blocks: usize,
}

impl FragmentationStats {
    const fn new() -> Self {
        FragmentationStats {
            total_free: 0,
            fragmented_blocks: 0,
        }
    }
}""", """/// Fragmentation tracking for buddy allocator.
///
/// `min_free_pages` is the one live field: the low-water mark of free
/// pages since boot, read by `/ctl/sys/mem/lowwater` (SkyOS/ade
/// kernel-mem-lowwater.md). `total_free`/`fragmented_blocks` remain dead
/// scaffolding for the rewrite's incremental counters.
#[allow(dead_code)]
struct FragmentationStats {
    total_free: usize,
    fragmented_blocks: usize,
    min_free_pages: usize,
}

impl FragmentationStats {
    const fn new() -> Self {
        FragmentationStats {
            total_free: 0,
            fragmented_blocks: 0,
            min_free_pages: usize::MAX,
        }
    }
}""", "FragmentationStats struct")

    src = apply_one(src, """            self.add_block(PhysAddr::new(current), order);
            current += 4096 << order;
        }
    }""", """            self.add_block(PhysAddr::new(current), order);
            current += 4096 << order;
        }
        // Seed the low-water mark. Each add_region only grows the free
        // pool, so after the last region this converges to the boot total
        // - the correct initial value (NOT usize::MAX, which would make
        // /ctl/sys/mem/lowwater print garbage before the first alloc).
        self.update_low_water();
    }""", "add_region seed")

    src = apply_one(src, """        if let Some(addr) = self.free_lists[order] {
            self.free_lists[order] = self.read_next_ptr(addr);
            return Some(addr);
        }""", """        if let Some(addr) = self.free_lists[order] {
            self.free_lists[order] = self.read_next_ptr(addr);
            self.update_low_water();
            return Some(addr);
        }""", "allocate direct-pop path")

    src = apply_one(src, """        let addr = self.allocate_at_order(order + 1)?;
        let buddy_addr = PhysAddr::new(addr.as_u64() + (4096 << order));
        self.add_block(buddy_addr, order);
        Some(addr)
    }""", """        let addr = self.allocate_at_order(order + 1)?;
        let buddy_addr = PhysAddr::new(addr.as_u64() + (4096 << order));
        self.add_block(buddy_addr, order);
        // The recursive call already caught the true minimum (free = F -
        // 2^(order+1)); this call is a no-op. Kept so every successful
        // allocate return path visibly maintains the invariant.
        self.update_low_water();
        Some(addr)
    }""", "allocate split path")

    src = apply_one(src, """    pub fn deallocate_at_order(&mut self, addr: PhysAddr, order: usize) {
        if order >= MAX_ORDER {""", """    pub fn deallocate_at_order(&mut self, addr: PhysAddr, order: usize) {
        // Free pages only grow here, so this is a no-op today; kept so
        // every free path (incl. any future eviction inside deallocate)
        // maintains the low-water invariant.
        self.update_low_water();
        if order >= MAX_ORDER {""", "deallocate top")

    src = apply_one(src, """        total
    }

    /// Get fragmentation ratio (0.0 = no fragmentation, 1.0 = highly fragmented)""", """        total
    }

    /// Low-water mark: recompute the free-page count and keep the minimum
    /// ever observed. Called on every successful allocate/deallocate and
    /// after each add_region (the boot seed). O(free-list walk) per call;
    /// the rewrite may replace it with the dead `total_free` counter.
    fn update_low_water(&mut self) {
        let free = self.count_free_pages();
        if free < self.stats.min_free_pages {
            self.stats.min_free_pages = free;
        }
    }

    /// Minimum free pages ever observed since boot. Monotonic
    /// non-increasing; == boot total before the first allocation.
    pub fn low_water(&self) -> usize {
        self.stats.min_free_pages
    }

    /// Get fragmentation ratio (0.0 = no fragmentation, 1.0 = highly fragmented)""", "update_low_water + low_water")
    return src


def transform_ctlfs(src):
    src = apply_one(src, """            add_child(&mem_dir, "free", file_fn(|| {
                let free = crate::memory::buddy::BUDDY_ALLOCATOR.lock().count_free_pages();
                alloc::format!("{} pages\\n", free).into_bytes()
            }));
            add_child(&mem_dir, "used", file_fn(|| {""", """            add_child(&mem_dir, "free", file_fn(|| {
                let free = crate::memory::buddy::BUDDY_ALLOCATOR.lock().count_free_pages();
                alloc::format!("{} pages\\n", free).into_bytes()
            }));
            add_child(&mem_dir, "lowwater", file_fn(|| {
                // Low-water mark since boot (min over all post-op states).
                // Unlike `free` it never rises, so the Option 1 vs 2
                // memory-pressure question can be answered from one boot.
                let low = crate::memory::buddy::BUDDY_ALLOCATOR.lock().low_water();
                alloc::format!("{} pages\\n", low).into_bytes()
            }));
            add_child(&mem_dir, "used", file_fn(|| {""", "ctlfs lowwater node")
    return src


HEADER = """# Kernel spec: `/ctl/sys/mem/lowwater` — free-page low-water mark

**Status:** SPEC ONLY — the kernel is under external major change; not applied.
**Date:** Aug 13, 2026. **Verified against:** live kernel tree
(`SKYIOUS KERNEL/kernel/src/memory/buddy.rs`, `.../vfs/ctlfs.rs`) via
difflib + `git apply --check` (same treatment as Option 2/2b in
kernel-gui-window-fix.md). **Regenerate:** `python ade/docs/gen_lowwater_patch.py`
(idempotent; re-writes this file). **Intended for:** the kernel rewrite to
pick up verbatim. **Host pin:** `TestOption2bDocDiff::test_lowwater_diff_applies_cleanly_to_kernel`
in `tests/test_login_flow.py` re-extracts this block and re-runs
`git apply --check` on every host-tests run, so the draft cannot drift from
the live source.

---

## 1. The gap

The Option 1 vs Option 2 decision in kernel-gui-window-fix.md rests on
whether the GUI-window allocation failure is *transient* or *persistent*.
Today the evidence is per-boot snapshots: login-manager and the getty print
`[login] mem free=N pages` from `/ctl/sys/mem/free` before each
`Window::create`, and `qemu_giveup_boot.exp` classifies a ≥2-reading series
(recovering = transient → Option 1; flat = persistent → Option 2). But
`free` is a point-in-time read — it answers "how much is free *now*", not
"how low did it *ever* go". The transient/persistent split genuinely needs
the **low-water mark**: the minimum free-page count observed since boot,
which is monotonic (never rises) and therefore summarizes a whole boot in a
single value.

Today there is no such signal:

- `kernel/src/memory/buddy.rs:11-22` — `FragmentationStats { total_free,
  fragmented_blocks }` is `#[allow(dead_code)]` scaffolding; neither field
  is ever updated.
- `kernel/src/vfs/ctlfs.rs:201-210` — `/ctl/sys/mem/` serves `total`,
  `free`, `used`, `cached`; `free` walks the free lists on demand. Nothing
  records a minimum.

## 2. Design

1. **`FragmentationStats.min_free_pages: usize`** (buddy.rs:11) — the one
   live field; `total_free`/`fragmented_blocks` stay dead scaffolding (the
   rewrite's incremental counters). Initialized to `usize::MAX` ("unset").
2. **`BuddyAllocator::update_low_water()`** — recompute `count_free_pages()`
   and keep the minimum ever seen. Called:
   - after each `add_region` (the boot **seed**: region adds only grow the
     pool, so after the last region the value converges to the boot total —
     NOT `usize::MAX`, which would print garbage from the node before the
     first allocation),
   - on every successful `allocate_at_order` return path,
   - at the top of `deallocate_at_order`.
3. **`BuddyAllocator::low_water() -> usize`** getter, consumed by ctlfs.
4. **`/ctl/sys/mem/lowwater`** node (ctlfs.rs:205) — next to `free`, same
   `"{} pages\\n"` shape.

## 3. Patch (difflib-generated, `git apply --check` verified)

```diff
"""

NOTES = """```

## 4. Notes / caveats

- SPEC-ONLY, **not compiled**: the bar is difflib-correctness +
  `git apply --check`, same as Option 2's kernel hunk; the rewrite must
  compile-check when it lands.
- **Apply-equivalence:** the generator (`ade/docs/gen_lowwater_patch.py`)
  additionally applies the patch to scratch copies of both kernel files
  and byte-compares the result against the intended transform, so an
  apply-clean-but-wrong hunk (e.g. a brace misaligned by the diff
  heuristic) is caught at regeneration time, not just at git-apply time.
- The split-path `update_low_water()` after `add_block` is technically a
  no-op — the recursive `allocate_at_order(order + 1)` already captured the
  true minimum at `free = F - 2^(order+1)` — kept so every successful
  allocate return path visibly calls the helper.
- `deallocate_at_order` only grows free, so its call is a no-op today; kept
  defensively (any future free path inside deallocate inherits the
  invariant).
- **Not captured (out of scope):** the momentary dip inside
  `allocate_frame`'s swap-eviction retry — `try_evict_one_page` frees a
  page outside `allocate_at_order`. The rewrite may call `update_low_water`
  there to close the gap.
- `count_free_pages()` is O(free-list walk); the per-op update cost is the
  same order. The rewrite should wire the dead `total_free` counter
  incrementally (add on deallocate, subtract on allocate) and derive the
  low-water from it.
- Userspace consumers are unchanged by this patch: the `[login] mem free=N`
  markers keep printing from `/ctl/sys/mem/free`. A follow-up (not part of
  this patch) can add `[login] mem lowwater=N` reads so `qemu_giveup_boot.exp`
  classifies from one boot; landing condition K7 in session-lifecycle.md.
"""


def build_doc(buddy_diff, ctlfs_diff):
    body = HEADER
    for d in (buddy_diff, ctlfs_diff):
        body += "\n".join("    " + ln for ln in d.splitlines()) + "\n"
    return body + NOTES


def main():
    root = kernel_root()
    if not root:
        sys.exit("kernel tree not found (SKYOS_KERNEL_DIR or SKYIOUS KERNEL sibling)")
    buddy_p = os.path.join(root, "kernel", "src", "memory", "buddy.rs")
    ctlfs_p = os.path.join(root, "kernel", "src", "vfs", "ctlfs.rs")
    # Normalize CRLF -> LF BEFORE transforming: the anchors below are
    # written in LF, and buddy.rs is CRLF (ctlfs.rs is LF). The kernel
    # working tree is never written; the diff stays LF, which git apply
    # tolerates against CRLF trees (proven by the Option 2 kernel hunk).
    buddy = normalize(read(buddy_p))
    ctlfs = normalize(read(ctlfs_p))

    buddy_diff = patch_for(buddy, transform_buddy(buddy), "kernel/src/memory/buddy.rs")
    ctlfs_diff = patch_for(ctlfs, transform_ctlfs(ctlfs), "kernel/src/vfs/ctlfs.rs")
    doc = build_doc(buddy_diff, ctlfs_diff)

    # git apply --check against the live kernel tree (bytes, not text: the
    # Windows pipe would translate \n -> \r\n and corrupt the patch).
    patch = (buddy_diff + ctlfs_diff).encode("utf-8")
    r = subprocess.run(
        ["git", "apply", "--check", "--whitespace=nowarn", "-"],
        cwd=root,
        input=patch,
        capture_output=True,
    )
    if r.returncode != 0:
        sys.exit("git apply --check FAILED (doc NOT written): %s"
                 % r.stderr.decode("utf-8", "replace").strip()[:500])

    # Apply-equivalence: applying the patch to scratch copies of the two
    # kernel files must produce EXACTLY the transformed content. This closes
    # the class of bug where a diff applies cleanly but lands something other
    # than what the transform intended (e.g. a brace misaligned by the
    # diffing heuristic). The scratch dir is disposable; the kernel tree is
    # never touched.
    import tempfile
    scratch = tempfile.mkdtemp(prefix="lowwater_apply_")
    try:
        for rel, src, want in (
            ("kernel/src/memory/buddy.rs", buddy, transform_buddy(buddy)),
            ("kernel/src/vfs/ctlfs.rs", ctlfs, transform_ctlfs(ctlfs)),
        ):
            dst = os.path.join(scratch, rel)
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            # Preserve the original bytes (CRLF for buddy.rs) so the patch
            # applies under the same conditions as the live tree.
            shutil.copyfile(os.path.join(root, rel), dst)
        q = subprocess.run(
            ["git", "apply", "--whitespace=nowarn", "-"],
            cwd=scratch,
            input=patch,
            capture_output=True,
        )
        if q.returncode != 0:
            sys.exit("scratch git apply FAILED: %s"
                     % q.stderr.decode("utf-8", "replace").strip()[:500])
        for rel, _, want in (
            ("kernel/src/memory/buddy.rs", buddy, transform_buddy(buddy)),
            ("kernel/src/vfs/ctlfs.rs", ctlfs, transform_ctlfs(ctlfs)),
        ):
            applied = normalize(io.open(os.path.join(scratch, rel),
                                        encoding="utf-8", newline="").read())
            if applied != want:
                sys.exit("apply-equivalence FAILED for %s: applied != transform"
                         % rel)
    finally:
        shutil.rmtree(scratch, ignore_errors=True)
    with io.open(DOC, "w", encoding="utf-8", newline="\n") as f:
        f.write(doc)

    # Idempotency: regenerating from the same source reproduces the file.
    again = build_doc(
        patch_for(buddy, transform_buddy(buddy), "kernel/src/memory/buddy.rs"),
        patch_for(ctlfs, transform_ctlfs(ctlfs), "kernel/src/vfs/ctlfs.rs"),
    )
    if again != doc:
        sys.exit("generator NOT idempotent - aborting (doc left on disk)")
    print("git apply --check OK; wrote", DOC)
    print("buddy.rs hunks: %d lines, ctlfs.rs hunks: %d lines"
          % (len(buddy_diff.splitlines()), len(ctlfs_diff.splitlines())))
    return 0


if __name__ == "__main__":
    sys.exit(main())
