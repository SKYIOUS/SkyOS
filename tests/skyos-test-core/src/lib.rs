use std::time::Instant;

/// A registered test function.
pub struct Test {
    pub name: &'static str,
    pub category: &'static str,
    pub run: Box<dyn Fn() -> Result<(), String>>,
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
    fn from_test(test: &Test, result: Result<(), String>, duration_ms: u64) -> Self {
        let (passed, message) = match result {
            Ok(()) => (true, String::new()),
            Err(e) => (false, e),
        };
        TestRun {
            name: test.name.to_string(),
            category: test.category.to_string(),
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
}

impl TestRunner {
    pub fn new() -> Self {
        TestRunner { tests: Vec::new(), runs: Vec::new() }
    }

    pub fn register(&mut self, test: Test) {
        self.tests.push(test);
    }

    pub fn register_all(&mut self, tests: Vec<Test>) {
        self.tests.extend(tests);
    }

    pub fn run_all(&mut self) {
        self.runs.clear();
        for test in &self.tests {
            let start = Instant::now();
            let result = (test.run)();
            let duration = start.elapsed().as_millis() as u64;
            self.runs.push(TestRun::from_test(test, result, duration));
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
}

pub mod mock;
pub mod suites;
