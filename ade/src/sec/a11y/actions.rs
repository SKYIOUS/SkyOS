#[derive(Clone, Copy, Debug)]
pub(crate) enum A11yAction {
    Focus(u32),
    Activate(u32),
    Select(u32),
    Scroll(u32, i32),
    Dismiss(u32),
}
