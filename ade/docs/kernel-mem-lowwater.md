# Kernel spec: `/ctl/sys/mem/lowwater` — free-page low-water mark

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
   `"{} pages\n"` shape.

## 3. Patch (difflib-generated, `git apply --check` verified)

```diff
    --- a/kernel/src/memory/buddy.rs
    +++ b/kernel/src/memory/buddy.rs
    @@ -6,11 +6,17 @@
     
     pub const MAX_ORDER: usize = 11; // Blocks up to 2^11 * 4096 = 8MB
     
    -/// Fragmentation tracking for buddy allocator
    +/// Fragmentation tracking for buddy allocator.
    +///
    +/// `min_free_pages` is the one live field: the low-water mark of free
    +/// pages since boot, read by `/ctl/sys/mem/lowwater` (SkyOS/ade
    +/// kernel-mem-lowwater.md). `total_free`/`fragmented_blocks` remain dead
    +/// scaffolding for the rewrite's incremental counters.
     #[allow(dead_code)]
     struct FragmentationStats {
         total_free: usize,
         fragmented_blocks: usize,
    +    min_free_pages: usize,
     }
     
     impl FragmentationStats {
    @@ -18,6 +24,7 @@
             FragmentationStats {
                 total_free: 0,
                 fragmented_blocks: 0,
    +            min_free_pages: usize::MAX,
             }
         }
     }
    @@ -55,6 +62,11 @@
                 self.add_block(PhysAddr::new(current), order);
                 current += 4096 << order;
             }
    +        // Seed the low-water mark. Each add_region only grows the free
    +        // pool, so after the last region this converges to the boot total
    +        // - the correct initial value (NOT usize::MAX, which would make
    +        // /ctl/sys/mem/lowwater print garbage before the first alloc).
    +        self.update_low_water();
         }
     
         pub fn allocate_frame(&mut self) -> Option<PhysFrame> {
    @@ -87,6 +99,7 @@
     
             if let Some(addr) = self.free_lists[order] {
                 self.free_lists[order] = self.read_next_ptr(addr);
    +            self.update_low_water();
                 return Some(addr);
             }
     
    @@ -94,6 +107,10 @@
             let addr = self.allocate_at_order(order + 1)?;
             let buddy_addr = PhysAddr::new(addr.as_u64() + (4096 << order));
             self.add_block(buddy_addr, order);
    +        // The recursive call already caught the true minimum (free = F -
    +        // 2^(order+1)); this call is a no-op. Kept so every successful
    +        // allocate return path visibly maintains the invariant.
    +        self.update_low_water();
             Some(addr)
         }
     
    @@ -102,6 +119,10 @@
         }
     
         pub fn deallocate_at_order(&mut self, addr: PhysAddr, order: usize) {
    +        // Free pages only grow here, so this is a no-op today; kept so
    +        // every free path (incl. any future eviction inside deallocate)
    +        // maintains the low-water invariant.
    +        self.update_low_water();
             if order >= MAX_ORDER {
                 self.add_block(addr, order);
                 return;
    @@ -155,6 +176,23 @@
                 }
             }
             total
    +    }
    +
    +    /// Low-water mark: recompute the free-page count and keep the minimum
    +    /// ever observed. Called on every successful allocate/deallocate and
    +    /// after each add_region (the boot seed). O(free-list walk) per call;
    +    /// the rewrite may replace it with the dead `total_free` counter.
    +    fn update_low_water(&mut self) {
    +        let free = self.count_free_pages();
    +        if free < self.stats.min_free_pages {
    +            self.stats.min_free_pages = free;
    +        }
    +    }
    +
    +    /// Minimum free pages ever observed since boot. Monotonic
    +    /// non-increasing; == boot total before the first allocation.
    +    pub fn low_water(&self) -> usize {
    +        self.stats.min_free_pages
         }
     
         /// Get fragmentation ratio (0.0 = no fragmentation, 1.0 = highly fragmented)
    --- a/kernel/src/vfs/ctlfs.rs
    +++ b/kernel/src/vfs/ctlfs.rs
    @@ -202,6 +202,13 @@
                     let free = crate::memory::buddy::BUDDY_ALLOCATOR.lock().count_free_pages();
                     alloc::format!("{} pages\n", free).into_bytes()
                 }));
    +            add_child(&mem_dir, "lowwater", file_fn(|| {
    +                // Low-water mark since boot (min over all post-op states).
    +                // Unlike `free` it never rises, so the Option 1 vs 2
    +                // memory-pressure question can be answered from one boot.
    +                let low = crate::memory::buddy::BUDDY_ALLOCATOR.lock().low_water();
    +                alloc::format!("{} pages\n", low).into_bytes()
    +            }));
                 add_child(&mem_dir, "used", file_fn(|| {
                     let free = crate::memory::buddy::BUDDY_ALLOCATOR.lock().count_free_pages();
                     let total: usize = 512 * 1024 * 1024 / 4096;
```

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
