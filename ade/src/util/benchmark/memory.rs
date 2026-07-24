#![allow(dead_code)]

use crate::util::benchmark::BenchmarkResult;
use crate::core::desktop::Desktop;
use libsarga::io;

// ponytail: stub — add real memory measurement when kernel exposes heap stats
pub(crate) fn bench_memory_usage(_desktop: &mut Desktop) -> BenchmarkResult {
    io::print_str("[bench] memory_usage: stub\n");
    BenchmarkResult {
        name: "memory_usage",
        duration_ticks: 0,
        allocation_count: 0,
        memory_delta: 0,
    }
}
