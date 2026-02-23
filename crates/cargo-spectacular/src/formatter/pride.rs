use super::{FailedTest, Formatter, fg, reset, write_colored_failures, write_colored_summary};
use crate::event::SuiteResult;
use crossterm::terminal;
use std::f64::consts::TAU;
use std::io::{self, Write};

/// Sine-wave rainbow color, inspired by minitest's PrideLOL.
/// Walks red, green, blue around a circle separated by equal thirds.
fn rainbow_color(index: usize) -> (u8, u8, u8) {
    let frequency = 1.0 / 6.0;
    let n = index as f64 * frequency;
    let offset = TAU / 3.0;
    let r = (n.sin() * 127.0 + 128.0) as u8;
    let g = ((n + offset).sin() * 127.0 + 128.0) as u8;
    let b = ((n + 2.0 * offset).sin() * 127.0 + 128.0) as u8;
    (r, g, b)
}

pub struct PrideFormatter {
    dot_count: usize,
    cols: u16,
    failures: Vec<FailedTest>,
}

impl PrideFormatter {
    pub fn new() -> Self {
        let cols = terminal::size().map(|(w, _)| w).unwrap_or(80);
        Self {
            dot_count: 0,
            cols,
            failures: Vec::new(),
        }
    }

    fn emit_dot(&mut self, ch: char, w: &mut dyn Write) -> io::Result<()> {
        if self.dot_count > 0 && self.dot_count.is_multiple_of(self.cols as usize) {
            writeln!(w)?;
        }
        let (r, g, b) = rainbow_color(self.dot_count);
        fg(w, r, g, b)?;
        write!(w, "{ch}")?;
        reset(w)?;
        w.flush()?;
        self.dot_count += 1;
        Ok(())
    }

    fn write_rainbow_text(&self, text: &str, w: &mut dyn Write) -> io::Result<()> {
        for (i, ch) in text.chars().enumerate() {
            let (r, g, b) = rainbow_color(i);
            fg(w, r, g, b)?;
            write!(w, "{ch}")?;
        }
        reset(w)?;
        Ok(())
    }
}

impl Formatter for PrideFormatter {
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

        // "Ran N fabulous tests in X.XXXXs" — rainbow prefix, plain time
        let total = result.passed + result.failed + result.ignored;
        if let Some(t) = result.exec_time {
            let prefix = format!("Ran {total} fabulous tests in ");
            self.write_rainbow_text(&prefix, w)?;
            let tests_per_sec = total as f64 / t;
            writeln!(w, "{t:.4}s, {tests_per_sec:.1} tests/s")?;
        } else {
            let prefix = format!("Ran {total} fabulous tests");
            self.write_rainbow_text(&prefix, w)?;
            writeln!(w)?;
        }

        writeln!(w)?;
        write_colored_summary(result, w)?;
        writeln!(w)?;
        write_colored_failures(&self.failures, w)?;

        w.flush()?;
        Ok(())
    }
}
