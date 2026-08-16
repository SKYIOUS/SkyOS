use crate::Test;

pub mod kernel_mouse;
pub mod kernel_alloc;
pub mod kernel_vfs;
pub mod kernel_futex;
pub mod kernel_paging;

pub fn all() -> Vec<Test> {
    let mut tests = Vec::new();
    tests.extend(kernel_mouse::tests());
    tests.extend(kernel_alloc::tests());
    tests.extend(kernel_vfs::tests());
    tests.extend(kernel_futex::tests());
    tests.extend(kernel_paging::tests());
    tests
}
