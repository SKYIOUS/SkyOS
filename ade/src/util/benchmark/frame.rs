#![allow(dead_code)]

use crate::core::desktop::Desktop;
use crate::util::benchmark::BenchmarkResult;
use libsarga::io;

// ponytail: stub — add real frame timing when rendering pipeline is measured
pub(crate) fn bench_frame_rate(_desktop: &mut Desktop) -> BenchmarkResult {
    io::print_str("[bench] frame_rate: stub\n");
    BenchmarkResult {
        name: "frame_rate",
        duration_ticks: 0,
        allocation_count: 0,
        memory_delta: 0,
    }
}
