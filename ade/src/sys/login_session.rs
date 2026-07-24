//! Login session — user session lifecycle, startup, shutdown, restart.
#![allow(dead_code)]

use alloc::string::String;

pub(crate) enum SessionState {
    LoggedOut,
    Active,
    ShuttingDown,
}

pub(crate) struct LoginSession {
    pub state: SessionState,
    pub username: String,
    pub login_time: u64,
}

impl LoginSession {
    pub fn new() -> Self {
        LoginSession {
            state: SessionState::Active,
            username: String::from("user"),
            login_time: 0,
        }
    }

    pub fn start(&mut self, username: &str, ticks: u64) {
        self.state = SessionState::Active;
        self.username = String::from(username);
        self.login_time = ticks;
    }

    pub fn shutdown(&mut self) {
        self.state = SessionState::ShuttingDown;
    }

    pub fn logout(&mut self) {
        self.state = SessionState::LoggedOut;
    }

    pub fn restart(&mut self) {
        self.state = SessionState::ShuttingDown;
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, SessionState::Active)
    }
}
