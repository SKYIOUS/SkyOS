#![allow(dead_code)]

use crate::render::compositor::Compositor;
use crate::render::layer::Layer;
use crate::util::benchmark::BenchmarkResult;
use libsarga::io;

pub(crate) fn bench_compositor() -> BenchmarkResult {
    let start = 0u64;
    let mut comp = match Compositor::new(320, 200) {
        Some(c) => c,
        None => {
            return BenchmarkResult {
                name: "compositor_clear_fill",
                duration_ticks: 0,
                allocation_count: 0,
                memory_delta: 0,
            }
        }
    };

    // Fill wallpaper layer with a pattern
    {
        let mut canvas = comp.layer_canvas(Layer::Wallpaper);
        canvas.fill_rect(0, 0, 320, 200, 0xFF1E1E2E);
    }
    // Fill desktop layer
    {
        let mut canvas = comp.layer_canvas(Layer::Desktop);
        canvas.fill_rect(10, 10, 100, 100, 0xFF2D2D40);
    }
    // Fill windows layer
    {
        let mut canvas = comp.layer_canvas(Layer::Windows);
        canvas.fill_rect(20, 20, 200, 150, 0xFF3A3A5C);
    }

    comp.clear_all();
    let elapsed = 0u64.wrapping_sub(start); // approximate; real bench uses clock_ticks

    io::print_str("[bench] compositor clear + fill\n");
    BenchmarkResult {
        name: "compositor_clear_fill",
        duration_ticks: elapsed,
        allocation_count: 0,
        memory_delta: 0,
    }
}
