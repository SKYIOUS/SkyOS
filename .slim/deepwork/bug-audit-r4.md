# Bug-Codemap Audit R4 — `SkyOS_Critical_Bug_Paths_and_Logical_Error_Traces_20260731_193411.codemap`

Audit method: systematic-debugging Phase 1 (root-cause) on every trace, per-claim
verification against source, fix only confirmed real bugs with smallest diff.
Verification: `cargo +nightly build --target x86_64-sarga.json --release` for each
touched crate + full workspace. No runtime (no QEMU GUI); builds are the check.

## Per-trace verdicts

| # | Claim | Verdict | Evidence |
|---|-------|---------|----------|
| 1 | slab allocator UAF/double-free | **PARTIAL — claim wrong, real leak** | Atomic swap prevents double-pop; popped block can't be re-freed mid-pop. Real bug: lost-block leak on concurrent push/push or push-during-pop (load→store window). |
| 2 | fork/exec fd leaks | **CONFIRMED — codemap understated** | Worse than "leak": pipeline never wires read-ends to next command (`prev_read = stdin` re-assigns the consumed value; `fds[0]` leaked every iteration AND `a \| b` never actually pipes). Regression since `3413321`. |
| 3 | glob/substitution | **3e CONFIRMED; rest FALSE** | do_glob slices `buf[off+20..off+20+namelen]` unbounded — same trust-boundary panic fixed earlier in libskyos list_dir. Depth counter blocks unterminated `$(`; glob is single-level; symlink/execvp nits match bash. |
| 4 | signal-handler races | **FALSE** | No SIGCHLD handler installed anywhere; sash polls waitpid. Theoretical only. |
| 5 | futex deadlock | **FALSE** | join/wait loops re-check the flag after FUTEX_WAIT; WAIT val-check makes missed-wakeup impossible. Standard correct pattern. |
| 6 | I/O error propagation | **6a/6b CONFIRMED; 6c minor; 6d FALSE** | cp.rs ignored write result (silent data loss on disk-full). dup2 unchecked — low value, noted. write()==0 on non-empty buf IS an error; EIO correct. |
| 7 | HTTP client | **7d/7e CONFIRMED; 7a-7c FALSE** | Unbounded response growth = memory DoS. `resolve([u8;4])` is a kernel contract, not a bug. Flagged: non-blocking socket reads may make `Err(_) => break` truncate large responses (kernel behavior unverifiable w/o QEMU). |
| 8 | init respawn race | **FALSE** | Single-threaded loop, pid cleared before respawn. Real nit (not fixed): crash counter never resets, so a service that crashes once after weeks still counts toward give-up. |

## Fixes applied (each verified with release build)

1. **sash/src/executor.rs** — pipeline wiring: track `pipe_read`, set
   `prev_read = pipe_read` (was `stdin`, the already-consumed old value); close
   `prev_read` on pipe-fail and `prev_read`+`pipe_write` on fork-fail. This both
   un-leaks the read end and makes `a | b | c` actually pipe.
2. **sash/src/executor.rs** — `do_glob`: bounds-check `offset + 20 > n` and
   `offset + 20 + namelen > n` before slicing (kernel-buffer trust boundary; same
   class as the earlier list_dir fix).
3. **coreutils/src/cp.rs** — `copy_file` now uses existing `io::write_all`
   (partial-write retry + error surface) and reports/cleans up on read/write failure.
4. **libsarga/src/mem.rs** — `SlabAllocator` free-list ops serialized with
   `RawMutex` (no allocation inside critical section; lock() is infinite-retry so
   the internal expect can't fire).
5. **libsarga/src/net.rs** — `HttpClient::get` caps response at 32MB (ENOSPC on
   exceed).
6. **init/src/main.rs** — execve failure now logs the errno instead of a bare
   "exec failed for X".

## Status
- Full workspace `--release` build: **Finished**.
- No new warnings in touched files (pre-existing dead_code warnings unchanged).
