use crate::Test;

pub mod kernel_mouse;
pub mod kernel_alloc;

pub fn all() -> Vec<Test> {
    let mut tests = Vec::new();
    tests.extend(kernel_mouse::tests());
    tests.extend(kernel_alloc::tests());
    tests
}
