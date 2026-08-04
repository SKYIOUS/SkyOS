#![allow(dead_code)]

use crate::core::desktop::Desktop;
use crate::util::benchmark::BenchmarkResult;
use libsarga::io;

// ponytail: stub — add real scheduler measurement when kernel exposes sched stats
pub(crate) fn bench_scheduler(_desktop: &mut Desktop) -> BenchmarkResult {
    io::print_str("[bench] scheduler: stub\n");
    BenchmarkResult {
        name: "scheduler",
        duration_ticks: 0,
        allocation_count: 0,
        memory_delta: 0,
    }
}
