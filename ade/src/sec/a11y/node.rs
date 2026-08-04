use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
// keep: role taxonomy for future widget roles
#[allow(dead_code)]
pub(crate) enum A11yRole {
    Desktop,
    Taskbar,
    StartMenu,
    Window,
    Button,
    TitleBar,
    Icon,
    MenuItem,
    Notification,
    Dialog,
    ScrollBar,
    TextInput,
    List,
    ListItem,
    Tooltip,
    Popup,
    Checkbox,
    Slider,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct A11yState {
    pub focused: bool,
    pub visible: bool,
    // keep: state flags, read by future focus/activation paths
    #[allow(dead_code)]
    pub enabled: bool,
    #[allow(dead_code)]
    pub selected: bool,
    #[allow(dead_code)]
    pub checked: Option<bool>,
}

#[derive(Clone, Debug)]
pub(crate) struct A11yNode {
    pub id: u32,
    pub role: A11yRole,
    pub label: String,
    pub bounds: (i32, i32, u32, u32),
    pub state: A11yState,
    pub focusable: bool,
    pub parent: Option<u32>,
    pub children: Vec<u32>,
}
