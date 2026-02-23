pub mod boring;
pub mod default;
pub mod pride;

use crate::event::SuiteResult;
use std::io::{self, Write};

/// Captured failure for replay in the summary.
pub struct FailedTest {
    pub name: String,
    pub exec_time: Option<f64>,
    pub stdout: Option<String>,
    pub message: Option<String>,
}

/// Pluggable test output formatter.
pub trait Formatter {
    fn suite_started(&mut self, test_count: usize, w: &mut dyn Write) -> io::Result<()>;
    fn test_started(&mut self, name: &str, w: &mut dyn Write) -> io::Result<()>;
    fn test_passed(
        &mut self,
        name: &str,
        exec_time: Option<f64>,
        w: &mut dyn Write,
    ) -> io::Result<()>;
    fn test_failed(
        &mut self,
        name: &str,
        exec_time: Option<f64>,
        stdout: Option<&str>,
        message: Option<&str>,
        w: &mut dyn Write,
    ) -> io::Result<()>;
    fn test_ignored(&mut self, name: &str, w: &mut dyn Write) -> io::Result<()>;
    fn suite_finished(
        &mut self,
        result: &SuiteResult,
        success: bool,
        w: &mut dyn Write,
    ) -> io::Result<()>;
}

/// Create a formatter by name.
pub fn create(name: &str) -> Box<dyn Formatter> {
    match name {
        "pride" => Box::new(pride::PrideFormatter::new()),
        "boring" => Box::new(boring::BoringFormatter::new()),
        "default" => Box::new(default::DefaultFormatter::new()),
        _ => {
            eprintln!("Unknown formatter: {name}, falling back to pride");
            Box::new(pride::PrideFormatter::new())
        }
    }
}

// ANSI helpers shared by colored formatters (pride, boring).

pub fn fg(w: &mut dyn Write, r: u8, g: u8, b: u8) -> io::Result<()> {
    write!(w, "\x1b[38;2;{r};{g};{b}m")
}

pub fn reset(w: &mut dyn Write) -> io::Result<()> {
    write!(w, "\x1b[0m")
}

const GREEN: (u8, u8, u8) = (100, 200, 120);
const RED: (u8, u8, u8) = (210, 90, 90);
const YELLOW: (u8, u8, u8) = (200, 180, 80);

/// Write the colored summary line: green passed, red failed, yellow ignored.
pub fn write_colored_summary(result: &SuiteResult, w: &mut dyn Write) -> io::Result<()> {
    fg(w, GREEN.0, GREEN.1, GREEN.2)?;
    write!(w, "{} passed", result.passed)?;
    reset(w)?;
    write!(w, ", ")?;
    fg(w, RED.0, RED.1, RED.2)?;
    write!(w, "{} failed", result.failed)?;
    reset(w)?;
    write!(w, ", ")?;
    fg(w, YELLOW.0, YELLOW.1, YELLOW.2)?;
    write!(w, "{} ignored", result.ignored)?;
    reset(w)?;
    writeln!(w)?;
    Ok(())
}

/// Replay captured failures with red coloring.
pub fn write_colored_failures(failures: &[FailedTest], w: &mut dyn Write) -> io::Result<()> {
    if failures.is_empty() {
        return Ok(());
    }
    writeln!(w, "Failures:\n")?;
    for (i, fail) in failures.iter().enumerate() {
        fg(w, RED.0, RED.1, RED.2)?;
        write!(w, "  {}. {}", i + 1, fail.name)?;
        if let Some(t) = fail.exec_time {
            write!(w, " ({t:.2}s)")?;
        }
        reset(w)?;
        writeln!(w)?;

        if let Some(ref msg) = fail.message {
            writeln!(w, "     {msg}")?;
        }
        if let Some(ref stdout) = fail.stdout {
            let trimmed = stdout.trim();
            if !trimmed.is_empty() {
                writeln!(w, "     --- stdout ---")?;
                for line in trimmed.lines() {
                    writeln!(w, "     {line}")?;
                }
            }
        }
        writeln!(w)?;
    }
    Ok(())
}
