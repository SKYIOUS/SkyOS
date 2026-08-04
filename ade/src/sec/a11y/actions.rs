#[derive(Clone, Copy, Debug)]
// keep: a11y action queue scaffold for screen-reader activation
#[allow(dead_code)]
pub(crate) enum A11yAction {
    Focus(u32),
    Activate(u32),
    Select(u32),
    Scroll(u32, i32),
    Dismiss(u32),
}
