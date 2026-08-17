# Async/Await Execution Model Design

SkyOS uses Rust's async/await for select kernel operations, driven by a single cooperative executor.

## Why Async?

- Stackless coroutines (state machines) instead of full thread stacks
- Context switches only at explicit yield points
- Simplifies I/O waits that would otherwise need dedicated blocking threads

The general kernel (syscall handling, drivers, filesystem I/O) remains synchronous threaded code; async is used where a waiting task maps naturally onto a `Future` (e.g. the shell's scancode stream).

## The Executor (`task/executor.rs`)

A single `Executor` runs in a **dedicated kernel thread** (`run_async_tasks`, spawned from `main.rs` via `task::scheduler::spawn`). It is not per-CPU.

```rust
pub struct Executor { /* tasks: HashMap<TaskId, Task>, ready_queue: ArrayQueue<TaskId> */ }
impl Executor {
    pub fn new() -> Self;
    pub fn spawn(&mut self, task: Task) -> Result<(), &'static str>;
    pub fn run(&mut self) -> !;  // run_ready_tasks(); sleep_if_idle();
}
```

Tasks are woken by `Waker`s pushed onto the `ready_queue` (`ArrayQueue`); `run()` pops them, polls each to `Poll::Pending`, and sleeps when idle.

## Async Keyboard Stream

`task/keyboard.rs` exposes `ScancodeStream` (a `futures_util::Stream`) that yields scancodes to the shell, registering a `Waker` when empty and waking on input. This is the canonical async consumer.

## Interrupt to Async Bridge

Interrupt-driven sources (e.g. PS/2 scancodes) push onto lock-free queues and wake the executor's waker rather than doing work in interrupt context.

## Driver I/O

Drivers are **not** async — block devices (`BlockDevice`), network tokens, and storage I/O are synchronous. The async executor is for kernel-internal event streams, not a general driver I/O model.
