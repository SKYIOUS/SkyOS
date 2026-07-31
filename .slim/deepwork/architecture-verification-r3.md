# Architecture Verification Round 3 — external userspace codemap vs. code

Source: `~/.codeium/windsurf/codemaps/SkyOS_Userspace_Architecture_20260731_191147.codemap`
(10 traces, ~50 location cites). Verified every cited line/symbol against source as of this session.
The .codemap file itself was NOT edited (per instruction).

## Traces — verified

1. **Init boot/respawn (ACCURATE)** — init/src/main.rs:60 entry, :64-75 mounts (tmpfs/devfs/ctlfs),
   :33 fork / :36 execve in Service::spawn, :94 spawn loop, :103 waitpid(-1,0) monitor, :113 respawn
   gate, :114 crash counter, :115 `> MAX_RESPAWNS` (5), :121 500ms backoff. All line cites hit.
2. **Syscall path (ACCURATE)** — io.rs:102 `read`, :105-108 error conversion; syscall.rs:183 wrapper,
   :7 `asm!` / :8 `syscall` instr, :5 `syscall6`, register convention rax/rdi/rsi/rdx/r10/r8/r9, SYS_READ=0.
3. **ADE event loop/render (ACCURATE)** — main.rs:24 `Window::create("SARGA OS Desktop", 800, 600)`,
   :33 `Compositor::new` (now `Option<Self>`), :52 tick, :54 get_key, :61 handle_event, :65 get_mouse,
   :84 dirty gate, :86 snapshot, :87 render, :88 flush, :104 adaptive 16.6ms pacing.
4. **sash shell (STALE, directionally right)** — codemap cites `readline::readline` (main.rs:265),
   `parser::parse(&expanded)` (:280), `executor::execute()` (executor.rs:15), fork :45 / execve :89 /
   waitpid :134. Reality: readline is inlined (`read_with_continuation`/`read_raw_line`, main.rs:466-476);
   parse+execute at main.rs:506-507 via `executor::execute_pipelines` (executor.rs:11); fork at
   executor.rs:73; execve at builtins.rs:155; job-control waitpid around main.rs:449-463. Module
   structure (readline.rs/parser.rs/executor.rs/scripting.rs) exists but names/lines drifted.
5. **Thread spawn/clone/futex (ACCURATE)** — thread.rs:25 spawn, :26 clear_tid Box<AtomicU32>, :30-34
   stack alloc (1MB/4096), :39 flags (CLONE_VM|CLONE_SETTLS|CLONE_CHILD_CLEARTID|CLONE_PARENT_SETTID),
   :44 syscall6(SYS_CLONE=56), :67-74 join via futex on clear_tid. Note: codemap's "parent_tid" arg slot
   is actually func_ptr in the current call; minor.
6. **GUI window lifecycle (STALE line cites, correct syscall numbers)** — gui.rs:7/8/9 consts 100/102/103
   correct; but create is gui.rs:421 (not :565), map_buffer :436, flush :474 (`syscall2(102, id,
   buffer_ptr)` — passes the buffer pointer, not 0 as the codemap claims), pixel write via
   draw_pixel/draw_rect (gui.rs:739+, not :650).
7. **spkg install flow (STALE, directionally right)** — no `resolve_dependencies`/`extract_package`/
   `topological_sort`; deps.rs uses `resolve`/`resolve_all` (deps.rs:40/53, DFS + order push), install.rs
   has `download_package` (:9), `extract_tar` (:39), `install_package` (:77); db.rs `save_db` (:37);
   "install" dispatch at main.rs:273; package format is tar `.spkg` (accepts `.skp` alias), not a custom
   `.skp` binary format. Dep resolution → download → extract → db update flow is accurate.
8. **Memory allocator (ACCURATE)** — mem.rs:100 alloc, :102 slab fast path, :108 mmap fallback, :118
   dealloc_to_slab, :124 munmap; SLAB_SIZES [8..2048] (:44). Codemap's "futex-based synchronization"
   title is loose — the slab uses atomics, not futex; its guide text says the same.
9. **Net sockets/DNS (STALE line cites, correct syscall numbers)** — socket(41), resolve(200), connect(42),
   sendto(44), recvfrom(45) all match syscall.rs; but sendto/recvfrom live in `libskyos/net_ext.rs:71/94`
   (not net.rs:320/350), resolve at net.rs:209 (uses `syscall2(200, buf, out_ip)`, not the codemap's 3-arg
   form), socket free fn at net.rs:321, connect at :286/:369.
10. **sync futex mutex (ACCURATE)** — sync.rs:18 RawMutex::lock, :33 CAS fast path, :49 FUTEX_WAIT,
    :54 store 0 Release, :57 FUTEX_WAKE. Also has RwLock, Condvar, TlsKey, init_tls (codemap covers mutex
    only; RwLock named in title).

## Verdict

6/10 traces line-accurate; 4/10 (sash, gui, spkg, net) have drifted line cites/names but describe the
architecture correctly. No trace contradicts current behavior in a way that indicates a regression —
the drift is normal evolution (scripting, pipelines, net_ext split, tar-based spkg, GUI draw API).

## Real code fixes (from audit)

1. **spkg repo-lookup unwraps (FIXED)** — `spkg/src/main.rs:143` (`cmd_install`) and :233 (`cmd_upgrade`)
   both did `repos.iter().find(|r| r.name == *r).unwrap()`. The key comes from `get_all_index_entries()`
   (enabled repos only) while `repos` holds all repos — currently unreachable, but a repos.conf change
   between the two `load_repos()` calls would panic. Replaced both with `match` + skip with a message.
   Verified: `cargo +nightly build --target x86_64-sarga.json --release -p spkg` Finished (3 pre-existing
   warnings).
2. **libsarga `list_dir` getdents64 parse panics (FIXED)** — `libskyos.rs:71-72` did
   `buf[off..off+8].try_into().unwrap()` and `buf[off+16..off+18].try_into().unwrap()` on a
   kernel-provided buffer at a trust boundary; a truncated record would panic. Also `off += d_reclen`
   could spin forever on a zero-length record. Added `off + 18 > n` bounds check (break) and a
   `d_reclen == 0` break guard. Verified: libsarga release build Finished.

## Panic-path sweep (rest of userspace) — CLEAN

- init, login-manager, svc, sash, spkg, sargad, searchd(?), syslogd, vahid, skybuild: zero
  unwrap/expect/panic/unreachable in non-test code.
- libsarga remaining unwraps are all provably safe: `sync.rs:42/261` `start.unwrap()` guarded by
  `if let Some(timeout)`; `thread.rs:32/88` const `Layout::from_size_align(1MB, 4096)`; libskyos:74-75
  now in-bounds after fix; net.rs:533-578 + semver.rs:75-107 are `#[cfg(test)]`.
- coreutils CLI unwraps (`less.rs:10`, `more.rs:9`, `shuf.rs:26`, `sum.rs:15`, `tsort.rs:12`) all
  guarded by `args::argc() > 1` before `args::get(1).unwrap()` — cannot be None.
- nettools wget.rs:12-15 parses guarded by the `all()` u8-parse check + `parts.len()==4`; httpd.rs:96
  is a `"::1"` literal. Safe.
- ade `testing/*`, `sys/audio.rs:354`, `sys/display.rs:379`, `sys/network.rs:347` — test-only.
- `tests/skyos-test*` — test harness code.

## Deferred (unchanged from R2)

- Phase 2.1 regional damage wiring — needs visual verification.
- Phase 2.2 a11y incremental rebuild — medium risk, no visual check available.
- Kernel codemap placeholder — kernel is an external repo.
