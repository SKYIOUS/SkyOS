use crate::service::session::SessionManager;
use libsarga::io;

/// Pins the session-end protocol: a fresh session is running, `request_end`
/// marks the ending state (idempotently), and the logout exit code is the
/// clean 0 that `init` interprets as a graceful session end.
pub(crate) fn test_session_end_protocol() -> bool {
    let mut s = SessionManager::new(64);

    if s.is_ending() {
        io::print_str("[test] FAIL test_session_end_protocol: fresh session is ending\n");
        return false;
    }
    if s.uptime(0) != 0 {
        io::print_str("[test] FAIL test_session_end_protocol: uptime not 0 at boot\n");
        return false;
    }

    s.request_end();
    if !s.is_ending() {
        io::print_str("[test] FAIL test_session_end_protocol: request_end did not mark ending\n");
        return false;
    }
    if s.exit_code() != 0 {
        io::print_str("[test] FAIL test_session_end_protocol: logout exit code != 0\n");
        return false;
    }

    s.request_end(); // idempotent
    if !s.is_ending() {
        io::print_str("[test] FAIL test_session_end_protocol: second request_end unset ending\n");
        return false;
    }

    io::print_str("[test] PASS test_session_end_protocol\n");
    true
}
