use crate::service::session::SessionManager;
use libsarga::io;

/// Pins the session-end protocol: a fresh session is running, `request_end`
/// marks the ending state (idempotently), and the logout exit code is the
/// clean 0 that `init` interprets as a graceful session end.
///
/// The final leg pins the Esc-twin contract: re-entering `request_end` any
/// number of times cannot mutate the unwind. Idempotency is structural —
/// the body is a single monotonic store to a private bool, and `exit_code`
/// is a compile-time constant that reads no state — so a second Esc press
/// in the a11y arm (or any key during the near-miss sweep) observes exactly
/// the same `is_ending()`/`exit_code()` as the first press.
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
    // State-free exit code: before any end request, the unwind value is
    // already the clean 0 — it does not depend on `ending`.
    if s.exit_code() != 0 {
        io::print_str("[test] FAIL test_session_end_protocol: exit code nonzero before ending\n");
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

    // Re-entry storm: 8 more request_end calls (one per key press the
    // near-miss sweep can throw at the ending desktop) must leave both the
    // ending state and the exit code exactly as the first call did. Any
    // mutation — a reset `ending`, a changed exit code — breaks the
    // structural idempotency contract the a11y Esc arm relies on.
    let mut prior_ok = true;
    for _ in 0..8 {
        s.request_end();
        prior_ok &= s.is_ending() && s.exit_code() == 0;
    }
    if !prior_ok {
        io::print_str(
            "[test] FAIL test_session_end_protocol: re-entry mutated the ending state or exit code\n",
        );
        return false;
    }

    io::print_str("[test] PASS test_session_end_protocol\n");
    true
}
