#[derive(Clone, Copy)]
pub(crate) struct ProfilerCounter {
    pub value: u64,
}

impl ProfilerCounter {
    pub fn new() -> Self {
        ProfilerCounter { value: 0 }
    }
    #[allow(dead_code)] // profiler API
    pub fn inc(&mut self) {
        self.value += 1;
    }
    pub fn reset(&mut self) {
        self.value = 0;
    }
}
