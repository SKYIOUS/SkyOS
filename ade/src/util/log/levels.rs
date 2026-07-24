#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Fatal = 5,
}
