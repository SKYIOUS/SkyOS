use std::time::{Duration, Instant};

/// Default per-test timeout. A test that exceeds it fails as "timed out"
/// instead of stalling the run (and with it, CI).
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// A registered test function.
pub struct Test {
    pub name: &'static str,
    pub category: &'static str,
    /// `+ Send`: each test is moved onto its own thread so a hung test can be
    /// abandoned at the timeout instead of stalling the whole run.
    pub run: Box<dyn Fn() -> Result<(), String> + Send>,
}

/// Result of running a single test.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TestRun {
    pub name: String,
    pub category: String,
    pub passed: bool,
    pub message: String,
    pub duration_ms: u64,
}

impl TestRun {
    fn from_test(
        name: &'static str,
        category: &'static str,
        result: Result<(), String>,
        duration_ms: u64,
    ) -> Self {
        let (passed, message) = match result {
            Ok(()) => (true, String::new()),
            Err(e) => (false, e),
        };
        TestRun {
            name: name.to_string(),
            category: category.to_string(),
            passed,
            message,
            duration_ms,
        }
    }
}

/// Collects and runs tests, produces reports.
pub struct TestRunner {
    tests: Vec<Test>,
    runs: Vec<TestRun>,
    /// Per-test cap; `None` disables the timeout. A timed-out test is marked
    /// FAILED and its thread abandoned (leaked) -- the process still exits
    /// normally once the run finishes, so one hung test costs at most one
    /// timeout instead of a forever-stalled CI job.
    timeout: Option<Duration>,
}

impl TestRunner {
    pub fn new() -> Self {
        Self::new_with_timeout(DEFAULT_TIMEOUT_MS)
    }

    /// `timeout_ms == 0` disables the cap (for debugging a genuinely slow
    /// test); otherwise the per-test timeout is `timeout_ms` milliseconds.
    pub fn new_with_timeout(timeout_ms: u64) -> Self {
        let timeout = if timeout_ms == 0 {
            None
        } else {
            Some(Duration::from_millis(timeout_ms))
        };
        TestRunner { tests: Vec::new(), runs: Vec::new(), timeout }
    }

    pub fn register(&mut self, test: Test) {
        self.tests.push(test);
    }

    pub fn register_all(&mut self, tests: Vec<Test>) {
        self.tests.extend(tests);
    }

    pub fn run_all(&mut self) {
        self.runs.clear();
        // Consume the registered tests: each closure moves into its own thread.
        // (run_all is called once per runner; re-register before a second run.)
        let tests = std::mem::take(&mut self.tests);
        for test in tests {
            // Destructure first: `run` moves into its thread, while `name` /
            // `category` (both &'static str) stay available for the report.
            let Test { name, category, run } = test;
            let start = Instant::now();
            let result = run_with_timeout(run, self.timeout);
            let duration = start.elapsed().as_millis() as u64;
            self.runs.push(TestRun::from_test(name, category, result, duration));
        }
    }

    pub fn runs(&self) -> &[TestRun] { &self.runs }
    pub fn total(&self) -> usize { self.runs.len() }
    pub fn passed(&self) -> usize { self.runs.iter().filter(|r| r.passed).count() }
    pub fn failed(&self) -> usize { self.runs.iter().filter(|r| !r.passed).count() }

    pub fn report(&self) -> TestReport {
        TestReport {
            runs: self.runs.clone(),
            total: self.total(),
        }
    }
}

/// Serializable test report for JSON/HTML export.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TestReport {
    pub runs: Vec<TestRun>,
    pub total: usize,
}

impl TestReport {
    pub fn passed(&self) -> usize { self.runs.iter().filter(|r| r.passed).count() }
    pub fn failed(&self) -> usize { self.runs.iter().filter(|r| !r.passed).count() }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn to_html(&self) -> String {
        let mut html = String::new();
        html.push_str("<!DOCTYPE html><html><head><meta charset=\"utf-8\">");
        html.push_str("<title>SkyOS Test Report</title>");
        html.push_str("<style>");
        html.push_str("body{font-family:sans-serif;margin:2rem;background:#1e1e1e;color:#ccc}");
        html.push_str("h1{color:#fff}.pass{color:#4caf50}.fail{color:#f44336}");
        html.push_str(".summary{font-size:1.2rem;margin:1rem 0}");
        html.push_str("table{width:100%;border-collapse:collapse}");
        html.push_str("th,td{padding:8px 12px;text-align:left;border-bottom:1px solid #333}");
        html.push_str("th{background:#2d2d2d;color:#fff}");
        html.push_str(".badge{display:inline-block;padding:2px 8px;border-radius:4px;font-size:0.8rem}");
        html.push_str(".badge-pass{background:#1b5e20;color:#a5d6a7}");
        html.push_str(".badge-fail{background:#b71c1c;color:#ef9a9a}");
        html.push_str(".timestamp{color:#888;font-size:0.9rem}");
        html.push_str("</style></head><body>");
        html.push_str(&format!("<h1>SkyOS Test Report</h1>"));
        html.push_str(&format!("<div class=\"timestamp\">{}</div>", chrono_now()));
        html.push_str(&format!(
            "<div class=\"summary\">Total: {} | <span class=\"pass\">Passed: {}</span> | <span class=\"fail\">Failed: {}</span></div>",
            self.total, self.passed(), self.failed()));
        html.push_str("<table><tr><th>Test</th><th>Category</th><th>Result</th><th>Duration</th><th>Message</th></tr>");
        for run in &self.runs {
            let badge = if run.passed { "badge-pass" } else { "badge-fail" };
            let label = if run.passed { "PASS" } else { "FAIL" };
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td><span class=\"badge {}\">{}</span></td><td>{}ms</td><td>{}</td></tr>",
                run.name, run.category, badge, label, run.duration_ms, run.message));
        }
        html.push_str("</table></body></html>");
        html
    }
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = d.as_secs();
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    format!("{:02}:{:02}:{:02} UTC", h, m, s)
}

pub fn write_report_json(report: &TestReport, path: &str) -> std::io::Result<()> {
    std::fs::write(path, report.to_json())
}

pub fn write_report_html(report: &TestReport, path: &str) -> std::io::Result<()> {
    std::fs::write(path, report.to_html())
}

/// Assertions that return Result for test functions.
#[macro_export]
macro_rules! assert_result {
    ($cond:expr) => {
        if !$cond {
            return Err(format!("assertion failed: {}", stringify!($cond)));
        }
    };
    ($cond:expr, $($arg:tt)+) => {
        if !$cond {
            return Err(format!("{}: {}", format!($($arg)+), stringify!($cond)));
        }
    };
}

#[macro_export]
macro_rules! assert_eq_result {
    ($left:expr, $right:expr) => {{
        let l = $left;
        let r = $right;
        if l != r {
            return Err(format!(
                "assertion failed: `{} == {}`\n  left: {:?}\n right: {:?}",
                stringify!($left), stringify!($right), l, r));
        }
    }};
    ($left:expr, $right:expr, $($arg:tt)+) => {{
        let l = $left;
        let r = $right;
        if l != r {
            return Err(format!(
                "{}: assertion failed: `{} == {}`\n  left: {:?}\n right: {:?}",
                format!($($arg)+), stringify!($left), stringify!($right), l, r));
        }
    }};
}

/// Run one test closure on its own thread, catching panics and enforcing the
/// per-test timeout. Panics become failures; a timeout becomes a failure with
/// a "timed out" message and the thread is abandoned.
fn run_with_timeout(
    run: Box<dyn Fn() -> Result<(), String> + Send>,
    timeout: Option<Duration>,
) -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let spawn = std::thread::Builder::new().spawn(move || {
        // Flatten before sending: catch_unwind yields a nested Result; the
        // channel message must be the test's own `Result<(), String>` so the
        // receiver's `Ok(outcome)` unwraps directly.
        let outcome = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run())) {
            Ok(r) => r,
            Err(payload) => Err(panic_payload_message(payload)),
        };
        let _ = tx.send(outcome);
    });
    let spawn = match spawn {
        Ok(h) => h,
        Err(e) => return Err(format!("failed to spawn test thread: {}", e)),
    };
    // The channel carries the test's own Result; the recv error is the
    // timeout/vanished case. Never join the thread: a hung test must not
    // block the runner -- it is abandoned and reclaimed at process exit.
    drop(spawn);
    match timeout {
        Some(t) => match rx.recv_timeout(t) {
            Ok(outcome) => outcome,
            Err(_) => Err(format!("timed out after {}ms", t.as_millis())),
        },
        None => match rx.recv() {
            Ok(outcome) => outcome,
            Err(_) => Err("test thread vanished".to_string()),
        },
    }
}

fn panic_payload_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        return (*s).to_string();
    }
    if let Some(s) = payload.downcast_ref::<String>() {
        return s.clone();
    }
    "panicked (non-string payload)".to_string()
}

pub mod mock;
pub mod suites;

#[cfg(test)]
mod tests {
    use super::*;

    fn test(name: &'static str, f: impl Fn() -> Result<(), String> + Send + 'static) -> Test {
        Test { name, category: "self", run: Box::new(f) }
    }

    #[test]
    fn passing_test_reports_ok() {
        let mut r = TestRunner::new_with_timeout(1000);
        r.register(test("pass", || Ok(())));
        r.run_all();
        assert_eq!(r.total(), 1);
        assert_eq!(r.passed(), 1);
        assert_eq!(r.failed(), 0);
        assert_eq!(r.runs()[0].name, "pass");
        assert_eq!(r.runs()[0].message, "");
    }

    #[test]
    fn failing_test_reports_message() {
        let mut r = TestRunner::new_with_timeout(1000);
        r.register(test("fail", || Err("boom".to_string())));
        r.run_all();
        assert_eq!(r.failed(), 1);
        assert!(r.runs()[0].message.contains("boom"), "message: {}", r.runs()[0].message);
    }

    #[test]
    fn hung_test_times_out_and_is_failed() {
        let mut r = TestRunner::new_with_timeout(50);
        r.register(test("hang", || {
            std::thread::sleep(Duration::from_secs(30));
            Ok(())
        }));
        r.run_all();
        assert_eq!(r.failed(), 1);
        assert!(r.runs()[0].message.contains("timed out"), "message: {}", r.runs()[0].message);
        assert!(r.runs()[0].duration_ms >= 50, "duration: {}", r.runs()[0].duration_ms);
    }

    #[test]
    fn panicking_test_is_a_failure_not_an_abort() {
        let mut r = TestRunner::new_with_timeout(1000);
        r.register(test("panic", || panic!("kaboom")));
        r.run_all();
        assert_eq!(r.failed(), 1);
        assert!(r.runs()[0].message.contains("kaboom"), "message: {}", r.runs()[0].message);
    }

    #[test]
    fn zero_timeout_disables_the_cap() {
        let mut r = TestRunner::new_with_timeout(0);
        r.register(test("pass", || Ok(())));
        r.run_all();
        assert_eq!(r.passed(), 1);
    }

    #[test]
    fn all_tests_run_even_with_failures() {
        let mut r = TestRunner::new_with_timeout(1000);
        r.register(test("a", || Err("x".to_string())));
        r.register(test("b", || Ok(())));
        r.register(test("c", || Ok(())));
        r.run_all();
        assert_eq!(r.total(), 3);
        assert_eq!(r.passed(), 2);
        assert_eq!(r.failed(), 1);
    }
}
