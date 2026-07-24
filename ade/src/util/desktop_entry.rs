#![allow(dead_code)]

pub(crate) struct DesktopEntry {
    pub name: &'static str,
    pub exec: &'static str,
    pub icon: &'static str,
    pub categories: &'static [&'static str],
    pub permissions: u32,
    pub version: &'static str,
}
