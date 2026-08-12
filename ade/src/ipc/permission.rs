use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) struct AppPermission: u32 {
        const CLIPBOARD = 0x0001;
        const NOTIFICATIONS = 0x0002;
        const FILESYSTEM = 0x0004;
        const WINDOW_CONTROL = 0x0008;
        const SETTINGS = 0x0010;
        const POWER = 0x0020;
    }
}

pub(crate) const PERM_CLIPBOARD: AppPermission = AppPermission::CLIPBOARD;
pub(crate) const PERM_NOTIFICATIONS: AppPermission = AppPermission::NOTIFICATIONS;
pub(crate) const PERM_FILESYSTEM: AppPermission = AppPermission::FILESYSTEM;
pub(crate) const PERM_WINDOW_CONTROL: AppPermission = AppPermission::WINDOW_CONTROL;
pub(crate) const PERM_SETTINGS: AppPermission = AppPermission::SETTINGS;
