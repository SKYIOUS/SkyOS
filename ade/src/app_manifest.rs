#![allow(dead_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StartupMode {
    Manual,
    Auto,
    Background,
}

pub(crate) struct AppManifest {
    pub name: &'static str,
    pub id: &'static str,
    pub version: &'static str,
    pub author: &'static str,
    pub permissions: u32,
    pub entry_point: &'static str,
    pub icon: &'static str,
    pub category: &'static str,
    pub supported_protocols: &'static [&'static str],
    pub startup_mode: StartupMode,
}
