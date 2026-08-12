#[derive(Clone, Copy)]
pub(crate) struct ProfilerCounter {
    pub value: u64,
}

impl ProfilerCounter {
    pub fn new() -> Self {
        ProfilerCounter { value: 0 }
    }
}
