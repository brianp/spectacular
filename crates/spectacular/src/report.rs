use crate::types::{SuiteResult, TestOutcome};
use std::io::{self, Write};

fn use_color() -> bool {
    std::env::var("NO_COLOR").is_err()
}

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

fn styled(code: &str, text: &str) -> String {
    if use_color() {
        format!("{}{}{}", code, text, RESET)
    } else {
        text.to_string()
    }
}

pub fn print_header(seed: u64) {
    println!(
        "\n{} {}\n",
        styled(BOLD, &format!("spectacular v{}", env!("CARGO_PKG_VERSION"))),
        styled(DIM, &format!("(seed: {})", seed)),
    );
}

pub fn print_group(name: &str) {
    println!("{}", styled(BOLD, &format!("● {}", name)));
}

pub fn print_test_result(name: &str, outcome: &TestOutcome, duration: std::time::Duration) {
    let duration_ms = duration.as_secs_f64() * 1000.0;
    let timing = format!("({:.1}ms)", duration_ms);
    match outcome {
        TestOutcome::Passed => {
            println!("  {} {} {}", styled(GREEN, "✓"), name, styled(DIM, &timing),);
        }
        TestOutcome::Failed(msg) => {
            println!("  {} {} {}", styled(RED, "✗"), name, styled(DIM, &timing),);
            for line in msg.lines() {
                println!("    {}", styled(RED, line));
            }
        }
    }
}

pub fn print_summary(result: &SuiteResult) {
    let total = result.results.len();
    let passed = result.passed();
    let failed = result.failed();
    let total_ms = result.total_duration.as_secs_f64() * 1000.0;

    println!();

    if failed > 0 {
        println!(
            "{} {}, {}, {} total",
            styled(BOLD, "Results:"),
            styled(GREEN, &format!("{} passed", passed)),
            styled(RED, &format!("{} failed", failed)),
            total,
        );
    } else {
        println!(
            "{} {}, {} total",
            styled(BOLD, "Results:"),
            styled(GREEN, &format!("{} passed", passed)),
            total,
        );
    }

    println!("{} {:.1}ms", styled(BOLD, "Time:"), total_ms);
    println!(
        "{} {} (reproduce with TEST_SEED={})",
        styled(BOLD, "Seed:"),
        result.seed,
        result.seed,
    );
    println!();

    let _ = io::stdout().flush();
}
