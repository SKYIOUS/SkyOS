#![allow(dead_code)]

pub(crate) mod frame;
pub(crate) mod ipc;
pub(crate) mod memory;
pub(crate) mod renderer;
pub(crate) mod scheduler;
pub(crate) mod window;

pub(crate) struct BenchmarkResult {
    pub name: &'static str,
    pub duration_ticks: u64,
    pub allocation_count: usize,
    pub memory_delta: usize,
}

pub(crate) fn run_benchmarks(desktop: &mut crate::core::desktop::Desktop) -> alloc::vec::Vec<BenchmarkResult> {
    let mut results = alloc::vec::Vec::new();
    results.push(window::bench_create_destroy(desktop));
    results.push(renderer::bench_compositor());
    results.push(ipc::bench_message_roundtrip(desktop));
    results
}
