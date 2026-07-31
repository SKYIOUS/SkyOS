# Architecture Verification — docs vs. code

Task: verify every claim in `docs/arch.md` + `docs/architecture/*` + related docs against current code.
Kernel repo: `C:\Users\nanda\Desktop\Github\SKYIOUS KERNEL` (junction `kernel/`). Userspace repo: this dir.
Verified by direct read of source (no subagents).

## Kernel: claims that are TRUE

- Higher-half mapping at `0xFFFFFFFF80000000`, linker script `kernel/linker.ld`.
- Heap at `0xFFFF_C000_0000_0000`, phys offset `0xFFFF_8000_0000_0000` — addresses correct (sizes wrong, see below).
- Buddy allocator for physical frames (`memory/buddy.rs`), slab for small kernel objects (`memory/slab.rs` BLOCK_SIZES=[8..4096]).
- Global allocator: `Locked<FixedSizeBlockAllocator>` slab + `linked_list_allocator::LockedHeap` fallback (`allocator.rs`).
- Syscall ABI: `syscall` instr, rax=num, rdi/rsi/rdx/r10/r8/r9, return rax; `syscall_entry` via `global_asm!` + LSTAR (mod.rs:186, :5079).
- `panic = "abort"`, `#![deny(warnings)]`, `spin::Mutex` everywhere, `extern crate alloc`.
- COW fork (`clone_cow()` in memory/paging.rs), VMA list in process, `fd_table: Mutex<Vec<Option<FileDescriptor>>>`.
- Signals: `syscalls/signal.rs` — pending/blocked u64 bitmasks, 32 handlers, delivery in syscall postamble. TRUE.
- DAC via `check_file_permission()`; capability gates (CAP_NET_RAW=13 socket raw, CAP_SYS_ADMIN=21 mount, CAP_KILL kill, CAP_SETPCAP capset). TRUE.
- TLS: `wrfsbase`/MSR 0xC0000100, per-thread `fs_base`. TRUE.
- Per-CPU data via `gs:0x0` self-referential `PerCpuData`. TRUE.
- LSM hooks in open/mkdir/socket/kill/mount/execve (`security.rs`). TRUE.
- LAPIC timer preemptive scheduling, per-CPU ready state. TRUE.
- VFS trait-based (`VfsNode`, `FileSystem` traits), mount/unmount via `VfsManager`. TRUE (structure; types differ, see below).

## Kernel: claims that are FALSE / OUTDATED

1. **"Hybrid microkernel; FS/drivers/networking as userspace"** — FALSE. Monolithic kernel; every FS (ramfs, devfs, ctlfs, pipe, tarfs, skyfs, ext2, ext4, fat) and every driver (block/net/usb/audio/gpu) lives in `kernel/src`. `docs/overview.md` / `arch.md` are wrong.
2. **Scheduler "priority round-robin, 8 levels, LAPIC ~100Hz"** — OUTDATED. Now **stride scheduling**: `PassOrd(Box<Thread>)` BinaryHeap keyed by min `pass`; `Thread{ pass, stride, tickets (default 20), STRIDE_MAX=1<<20 }` (`task/scheduler.rs`, `task/thread.rs`). Legacy `ready_queues`/`flush_ready_queues` still present but `push_thread`+`mark_ready_queues_dirty` are `#[allow(dead_code)]`. `SCHEDULER.md`, `scheduling.md`, `arch.md` all outdated.
3. **Heap size 8 MiB ("Planned")** — actually `HEAP_SIZE = 128 MiB`, implemented (allocator.rs:9-10). `MEMORY_MAP.md` wrong.
4. **Ext2 read-only** — now read-write (`write_inode/write_block/write_file_blocks`, VfsNode::write at ext2.rs:741). VFS_DESIGN.md wrong.
5. **Only Tmpfs/Ext2-ro/FAT32/Pipe** — actually 8 FS: ramfs, pipe, tarfs, devfs, ctlfs, skyfs/, ext2 (rw), ext4, fat. VFS_DESIGN.md wrong.
6. **Global VFS `Arc<Mutex<...>>`** — actually `pub static VFS: SchedLock<VfsManager>` (mod.rs:388). Type differs.
7. **SMP trampoline at 0x7000** — actually `TRAMPOLINE_PHYS=0x8000` code + `DATA_PHYS=0x7000` data (smp.rs:12-13). smp.md wrong.
8. **Interrupt stacks IST1–IST7 for all handlers** — only double-fault uses IST (`DOUBLE_FAULT_IST_INDEX`, interrupts.rs:69). interrupts.md wrong.
9. **Time: "HPET primary, timer wheel, adjtimex, CLOCK_THREAD/PROCESS_CPUTIME"** — NONE exist. `sys_clock_gettime` supports only CLOCK_REALTIME(0, RTC) and CLOCK_MONOTONIC(1, LAPIC tick counter 10ms). No HPET, no adjtimex, no timer wheel. time.md wrong.
10. **IPC: "channel-based, RingBuffer<Message>, UIPC ports"** — NO `src/ipc/` module, no Channel/RingBuffer/IpcPort anywhere. Real IPC: AF_UNIX sockets (`net/unix.rs`), pipes, futex, eventfd, PTY. ipc.md wrong. Also no `src/kernel/` or `src/arch/x86_64/` dirs — layout in ARCHITECTURE.md is fictional.
11. **Drivers only PCI/AHCI/E1000/VirtIO** — actually storage: ahci, nvme, pata, virtio_block; net: e1000, virtio; usb: uhci, xhci, hid; audio: hda, pcspeaker; gpu: virtio_gpu; graphics: bga, console, psf. DRIVER_MODEL.md outdated. BlockDevice trait `read_sector(&mut self,..)->Result<(),BlockDeviceError>` matches doc. TRUE.
12. **Syscall count** — 152 dispatch arms (not "171" — numbers.rs defines 171 constants but ~152 handlers). Minor.
13. **Syscall numbers** — most POSIX-compatible: READ=0 WRITE=1 OPEN=2 CLOSE=3 STAT=4 FSTAT=5 LSEEK=8 MMAP=9 MPROTECT=10 MUNMAP=11 BRK=12 CLONE=56 FORK=57 EXECVE=59 EXIT=60 WAIT4=61 FUTEX=202 MOUNT=165 UMOUNT2=167 CLOCK_GETTIME=228 TIMES=352. GUI 100-105. Matches SYSCALL_ABI.md. TRUE.

## Kernel: undocumented architecture (exists, no docs)

- `memory/paging.rs`: `AddressSpace{pml4, virt_offset}` + `OffsetPageTable`, COW clone.
- `memory/stack.rs`: guard page (stack_bottom - 4096), `alloc_stack`/`free_stack`.
- `memory/`: buddy, frame_info, phys, pressure, slab, virt, aarch64.
- `task/thread.rs`: ThreadId(AtomicU64), TLS fs_base.
- `task/process.rs`: FileDescriptor enum (File/PtyMaster/PtySlave/EventFd), HandleTable.
- `net/unix.rs`: full AF_UNIX socket stack.
- `pty.rs`, `drivers/watchdog.rs`, `drivers/rtc.rs`, `drivers/serial.rs`.
- `syscalls/signal.rs` (implemented, underdocumented).

## ADE (docs/arch.md): claims vs. code

arch.md is a newer, more accurate doc. Verified:

- **1.1 unwraps: OUTDATED.** arch.md lists 6 (desktop.rs:547,709,1117,1433,1764; launcher.rs:121). Only **ONE remains**: `desktop.rs:1433` `explorer_id.unwrap()`. The other 5 were already fixed.
- **1.2 init respawn no crash-limit: TRUE.** `init/src/main.rs:106-108` — 500ms nanosleep + respawn, no counter.
- **1.3 WindowId stale index: TRUE.** `window_manager.rs:47` `WindowId(self.windows.len()-1)`; remove at :64/:79/:105. Index-based, unstable.
- **2.3 compositor OOM: TRUE.** `compositor.rs:652,666,678-680` — `vec![0u32; pixels]` LayerBuffer ×LAYER_COUNT(6) + 2 temp buffers, no fallback.
- **3.1 dual permission: TRUE.** `desktop.rs:137` PermissionManager + `sec/portal/*` registry required_permissions.
- **a11y full rebuild every tick: TRUE.** `desktop.rs:303` build_a11y_tree() each tick.
- **Boot flow: TRUE.** init mounts /tmp tmpfs, /dev devfs, /ctl ctlfs (:62-69); spawns login-manager + svc (respawn:true). ADE 800x600 "SARGA OS Desktop" + frame pacing via nanosleep (syscall 35).
- **Damage regions: PARTIALLY WRONG (better than doc).** `core/damage.rs` DamageTracker{add(Rect) merging, drain, mark_full, is_dirty} EXISTS and `compositor.rs:781` `compose(win, damage_rects: Option<&[Rect]>)` supports partial recompose (:784 `full = first_frame || map_or(true,is_empty)`, :810 partial path). BUT every caller still uses `mark_full()` (62 sites) and `render/mod.rs:208` calls `compose(win, None)`. So infra is written but **wired to full-recompose only** — the "no region support" claim is now FALSE; it's "region support exists, unused."
- **Naming**: doc calls it "Skyious kernel"; kernel header says "Vahi Kernel" (वाहि). Kernel codename = Vahi.

## Proposed improvements (ranked by impact/risk)

1. **Wire regional damage tracking** (HIGH impact, LOW risk — infra already written + tested pattern). Replace `compose(win, None)` with drained rects; convert a subset of `mark_full()` for small/frequent updates (cursor, window move) to `damage.add(rect)`. Infra (`DamageTracker`, partial compose) already exists and is unit-testable (37 existing `#[test]`s in ade).
2. **Fix remaining unwrap** desktop.rs:1433 (trivial, removes last panic site).
3. **Init respawn crash-limit** (init/main.rs:106-108) — crash-loop protection, small.
4. **WindowId stable counter** — medium refactor, many touchpoints.
5. **Update docs to match reality** — big doc sweep.

## Selected phase

**Phase: init respawn crash-limit** (arch.md item 1.2).

Rationale: the other high-impact items are either already done (1.1 unwraps) or risky to ship blind. Wiring the damage-rect compositor path (2.1) can't be visually verified in this env (no QEMU GUI run), so it stays deferred. Init crash-limit is a real, verified stability fix: a crashing service (login-manager/svc) previously respawned forever in a tight 500ms loop. Now `MAX_RESPAWNS = 5` per service.

### Status
- **DONE** — `init/src/main.rs`: added `MAX_RESPAWNS = 5` const, `crashes: u32` on `Service`, respawn logic decrements and gives up after threshold with `[init] giving up on <name>` log.
- **DONE** — removed last unwrap in ade: `desktop.rs:1433` `explorer_id.unwrap()` → `if let Some(exp_id)`.
- **DONE** — docs updated to reality: arch.md (scheduler, VFS, heap, driver list, microkernel→monolithic, damage-region status, unwrap+init items marked FIXED), SCHEDULER.md, MEMORY_MAP.md (128 MiB), ARCHITECTURE.md, VFS_DESIGN.md (SchedLock, 9 FS, RW ext2), architecture/{overview,smp,interrupts,time,ipc,memory,process,scheduling,sync}.md.
- **DONE** — both `init` and `ade` compile clean (release, x86_64-sarga).
- **VALIDATION** — full workspace build + kernel build pending.

### Deferred (documented, not built)
- Damage-rect regional recomposite wiring (infra exists in `core/damage.rs` + `compositor.rs:781`; needs visual verification).
- WindowId stable generation (window_manager.rs:47 index-based).
- Compositor OOM fallback (compositor.rs:652).
- Kernel doc truths that were NOT portable into this repo: kernel-side files (SKYIOUS KERNEL) have no stale docs to edit — the stale docs all live in `SkyOS/docs/` and are now fixed.
