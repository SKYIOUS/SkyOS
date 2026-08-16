//! End-to-end CLI contract tests against the real built binary.
//!
//! Pins the two CI-safety behaviors: (1) a misconfigured invocation --
//! unknown flag, unknown subcommand, stray positional -- must fail HARD
//! (non-zero exit), never be silently ignored; (2) the `--timeout-ms` flag
//! is accepted and a filtered run still works.
//!
//! The binary is invoked as a subprocess so these tests pin the actual
//! process exit code, not just clap's parse result.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_skyos-test"))
}

#[test]
fn unknown_flag_fails_hard() {
    let out = bin().args(["run", "--bogus-flag"]).output().expect("run binary");
    assert!(
        !out.status.success(),
        "unknown flag must fail: status {:?}",
        out.status
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--bogus-flag"), "usage must name the offending arg: {stderr}");
}

#[test]
fn unknown_subcommand_fails_hard() {
    let out = bin().arg("bogus-subcommand").output().expect("run binary");
    assert!(!out.status.success(), "unknown subcommand must fail: {:?}", out.status);
}

#[test]
fn stray_positional_fails_hard() {
    let out = bin().args(["run", "extra"]).output().expect("run binary");
    assert!(!out.status.success(), "stray positional must fail: {:?}", out.status);
}

#[test]
fn valid_filtered_run_succeeds() {
    let out = bin()
        .args(["run", "--category", "kernel::alloc", "--timeout-ms", "1000"])
        .output()
        .expect("run binary");
    assert!(out.status.success(), "valid run must succeed: {:?}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Total: 6 | Passed: 6 | Failed: 0"), "summary: {stdout}");
}

#[test]
fn timeout_flag_is_rejected_when_unparseable() {
    let out = bin().args(["run", "--timeout-ms", "abc"]).output().expect("run binary");
    assert!(!out.status.success(), "non-numeric timeout must fail: {:?}", out.status);
}
