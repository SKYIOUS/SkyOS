#![allow(dead_code)]

use alloc::vec::Vec;
use crate::util::benchmark::BenchmarkResult;
use crate::core::desktop::Desktop;
use crate::ipc::message::{IpcTarget, MessageBus};
use libsarga::io;

pub(crate) fn bench_message_roundtrip(desktop: &mut Desktop) -> BenchmarkResult {
    let n = 200;
    let start = desktop.clock_ticks;

    let mut bus = MessageBus::new();
    for _ in 0..n {
        let seq = bus.request(IpcTarget::Desktop, "ping", Vec::new());
        bus.respond(seq, true, Vec::new());
    }
    let drained = bus.drain();

    let elapsed = desktop.clock_ticks - start;
    io::print_str(&alloc::format!(
        "[bench] ipc_message_roundtrip: {} messages in {} ticks\n", n, elapsed
    ));
    BenchmarkResult {
        name: "ipc_message_roundtrip",
        duration_ticks: elapsed,
        allocation_count: drained.len(),
        memory_delta: 0,
    }
}
