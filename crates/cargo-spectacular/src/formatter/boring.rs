use super::{write_colored_failures, write_colored_summary, FailedTest, Formatter};
use crate::event::SuiteResult;
use crossterm::terminal;
use std::io::{self, Write};

pub struct BoringFormatter {
    dot_count: usize,
    cols: u16,
    failures: Vec<FailedTest>,
}

impl BoringFormatter {
    pub fn new() -> Self {
        let cols = terminal::size().map(|(w, _)| w).unwrap_or(80);
        Self {
            dot_count: 0,
            cols,
            failures: Vec::new(),
        }
    }

    fn emit_dot(&mut self, ch: char, w: &mut dyn Write) -> io::Result<()> {
        if self.dot_count > 0 && self.dot_count % self.cols as usize == 0 {
            writeln!(w)?;
        }
        write!(w, "{ch}")?;
        w.flush()?;
        self.dot_count += 1;
        Ok(())
    }
}

impl Formatter for BoringFormatter {
    fn suite_started(&mut self, test_count: usize, w: &mut dyn Write) -> io::Result<()> {
        writeln!(w, "\nRunning {test_count} tests\n")?;
        Ok(())
    }

    fn test_started(&mut self, _name: &str, _w: &mut dyn Write) -> io::Result<()> {
        Ok(())
    }

    fn test_passed(
        &mut self,
        _name: &str,
        _exec_time: Option<f64>,
        w: &mut dyn Write,
    ) -> io::Result<()> {
        self.emit_dot('.', w)
    }

    fn test_failed(
        &mut self,
        name: &str,
        exec_time: Option<f64>,
        stdout: Option<&str>,
        message: Option<&str>,
        w: &mut dyn Write,
    ) -> io::Result<()> {
        self.failures.push(FailedTest {
            name: name.to_string(),
            exec_time,
            stdout: stdout.map(String::from),
            message: message.map(String::from),
        });
        self.emit_dot('X', w)
    }

    fn test_ignored(&mut self, _name: &str, w: &mut dyn Write) -> io::Result<()> {
        self.emit_dot('*', w)
    }

    fn suite_finished(
        &mut self,
        result: &SuiteResult,
        _success: bool,
        w: &mut dyn Write,
    ) -> io::Result<()> {
        writeln!(w, "\n")?;

        let total = result.passed + result.failed + result.ignored;
        if let Some(t) = result.exec_time {
            let tests_per_sec = total as f64 / t;
            writeln!(w, "{total} tests run in {t:.4}s, {tests_per_sec:.1} tests/s")?;
        } else {
            writeln!(w, "{total} tests run")?;
        }

        writeln!(w)?;
        write_colored_summary(result, w)?;
        writeln!(w)?;
        write_colored_failures(&self.failures, w)?;

        w.flush()?;
        Ok(())
    }
}
