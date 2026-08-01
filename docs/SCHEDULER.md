# SkyOS Scheduler Design

This document describes the design of the SkyOS scheduler, covering its architecture, scheduling policy, and concurrency model.

## 1. Architecture

SkyOS uses a hybrid scheduling model:

1.  **Preemptive Multi-tasking:** For kernel `Thread`s, a preemptive stride (proportional-share) scheduler is used. Preemption is driven by the Local APIC (LAPIC) timer, which fires every 10ms.
2.  **Cooperative Multi-tasking:** For `async` tasks within a thread (e.g., the shell), a cooperative executor is used.

The global `SCHEDULER` is a `spin::Mutex` containing the scheduler state.

## 2. Preemptive Scheduler (`task/scheduler.rs`)

### 2.1 Scheduling Policy

-   **Stride Scheduling (proportional share):**
    -   Each thread carries `tickets` (default 20), `stride`, and `pass` (`STRIDE_MAX = 1<<20`).
    -   The scheduler keeps a max-heap (`stride_heap`) of `PassOrd(Box<Thread>)` ordered by minimum `pass`.
    -   `pick_next()` pops the lowest-`pass` thread from the heap, then work-steals from other CPUs' heaps (up to 3 attempts) before falling back to the global `pending_queue`.
    -   Wake paths (timer tick, `wake_pipe`, `wake_futex`, context-switch completion) stage threads into 8 priority-sorted `ready_queues` (priority 7 = highest); `pick_next()` flushes these into the `stride_heap` via `flush_ready_queues()` so the min-`pass` thread runs next. Only the direct `push_thread` heap path is dead code (`#[allow(dead_code)]`).
-   **Per-CPU state:** each CPU has its own `PerCpuScheduler` with a `stride_heap` and `current_thread`; newly spawned threads go to a global `pending_queue` and are stolen by idle CPUs.

### 2.2 Thread States

A `Thread` can be in one of the following states (`ThreadStatus`):

-   `Ready`: The thread is ready to run and is waiting in the `stride_heap` or `pending_queue`.
-   `Running`: The thread is currently executing on a CPU.
-   `Blocked`: The thread is waiting for an event (e.g., I/O, `sys_nanosleep`). Blocked threads are moved to a `sleep_queue`.
-   `Exited`: The thread has finished execution and is waiting to be reaped.

Global shared queues hold sleep/block/futex states; per-CPU schedulers read `crate::smp::get_cpu_id()`.

### 2.3 Context Switching

-   The `switch_context` function is implemented in raw x86_64 assembly.
-   It saves all general-purpose registers and the `rflags` register of the outgoing thread onto its stack.
-   It restores the registers of the incoming thread and uses a `ret` instruction to jump to its last known instruction pointer (`rip`).

## 3. SMP (Symmetric Multiprocessing)

-   The BSP (Bootstrap Processor) initializes the kernel and starts the APs (Application Processors) using the SIPI sequence.
-   Each AP performs its own initialization (GDT, IDT, LAPIC) and then enters the main scheduler loop (`task::scheduler::schedule()`).
-   Each core has its own `PerCpuScheduler`; scheduling state is not shared under one global mutex. Interrupt handlers interact with the scheduler via `try_lock` to avoid deadlocks.

## 4. Async Executor (`task/executor.rs`)

-   A cooperative executor based on `crossbeam-queue` runs `async` tasks.
-   It is designed to run within a single preemptive `Thread`.
-   The kernel shell, GUI updates, and network polling are implemented as async tasks.
-   This allows for efficient, non-blocking I/O within the kernel without needing to block an entire kernel thread.
