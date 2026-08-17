# Scheduler Design

The SkyOS scheduler is a **preemptive, proportional-share (stride) scheduler** per CPU, driven by the LAPIC timer (10ms tick). A separate async executor runs cooperative tasks in a dedicated kernel thread.

## Stride Scheduling

The core scheduling policy is stride scheduling (`task/scheduler.rs`):

- Each `Thread` has `tickets` (default 20), `stride`, and `pass`; `STRIDE_MAX = 1 << 20`.
- Each CPU keeps a `stride_heap` (BinaryHeap of `PassOrd(Box<Thread>)`) ordered by minimum `pass`.
- `pick_next()` pops the lowest-`pass` thread; on empty, it steals from other CPUs' heaps (up to 3 attempts), then from the global `pending_queue` of newly spawned threads.
- This gives proportional-share CPU allocation across threads (more tickets → larger stride → lower pass growth rate → runs more often).

New threads are spawned into a global `pending_queue`; any idle CPU steals from it. Threads blocked on sleep/futex/pipe/I/O sit in global shared queues and are woken into the runnable set.

## Async Executor

`task/executor.rs` runs cooperative async tasks (based on `crossbeam-queue`) inside a **single dedicated preemptive kernel thread** — it is not the primary scheduler. The kernel shell, GUI updates, and network polling can run as async tasks here.

## Context Switching

Context switches are performed by `switch_context` in raw x86_64 assembly: saves/restores all general-purpose registers + rflags onto the thread stack and `ret`s to the saved instruction pointer.
