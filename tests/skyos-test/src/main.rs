use clap::{Parser, Subcommand};
use skyos_test_core::{TestRunner, suites, write_report_json, write_report_html};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "skyos-test", about = "SkyOS Test Framework")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all available test suites
    List,
    /// Run tests
    Run {
        /// Filter by category (e.g. kernel::mouse, kernel::alloc)
        #[arg(short, long)]
        category: Option<String>,

        /// Output format: json, html, or console (default)
        #[arg(short, long, default_value = "console")]
        format: String,

        /// Output file path (for json/html)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Generate HTML report from existing JSON results
    Report {
        /// JSON results file
        #[arg(default_value = "skyos-test-results.json")]
        input: PathBuf,
        /// Output HTML file
        #[arg(short, long, default_value = "skyos-test-report.html")]
        output: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::List => {
            let tests = suites::all();
            println!("SkyOS Test Suites:");
            for test in &tests {
                println!("  [{:20}] {}", test.category, test.name);
            }
            println!("\nTotal: {} tests", tests.len());
        }
        Commands::Run { category, format, output } => {
            let mut runner = TestRunner::new();
            runner.register_all(suites::all());

            // Filter by category if specified
            if let Some(ref cat) = category {
                runner.register_all(
                    suites::all().into_iter()
                        .filter(|t| t.category.contains(cat.as_str()))
                        .collect()
                );
            }

            runner.run_all();
            let report = runner.report();

            match format.as_str() {
                "json" => {
                    let json = report.to_json();
                    if let Some(path) = output {
                        let _ = write_report_json(&report, path.to_str().unwrap_or("results.json"));
                        println!("Report written to {:?}", path);
                    } else {
                        println!("{}", json);
                    }
                }
                "html" => {
                    let path = output.unwrap_or_else(|| PathBuf::from("skyos-test-report.html"));
                    let _ = write_report_html(&report, path.to_str().unwrap());
                    println!("Report written to {:?}", path);
                }
                _ => {
                    // Console output
                    println!("\n=== SkyOS Test Results ===\n");
                    for run in runner.runs() {
                        let status = if run.passed { "PASS" } else { "FAIL" };
                        println!("  [{}] {} [{:20}] {}", status, run.name, run.category, run.duration_ms);
                        if !run.message.is_empty() {
                            println!("       {}", run.message);
                        }
                    }
                    println!("\nTotal: {} | Passed: {} | Failed: {}",
                        runner.total(), runner.passed(), runner.failed());
                }
            }
        }
        Commands::Report { input, output } => {
            let json_str = std::fs::read_to_string(&input)
                .expect("Failed to read input file");
            let report: skyos_test_core::TestReport = serde_json::from_str(&json_str)
                .expect("Failed to parse JSON");
            write_report_html(&report, output.to_str().unwrap())
                .expect("Failed to write report");
            println!("Report written to {:?}", output);
        }
    }
}
