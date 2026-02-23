use super::{FailedTest, Formatter};
use crate::event::SuiteResult;
use std::io::{self, Write};

/// No-color formatter for piped/non-TTY output.
pub struct DefaultFormatter {
    dot_count: usize,
    failures: Vec<FailedTest>,
}

impl DefaultFormatter {
    pub fn new() -> Self {
        Self {
            dot_count: 0,
            failures: Vec::new(),
        }
    }

    fn emit_dot(&mut self, ch: char, w: &mut dyn Write) -> io::Result<()> {
        if self.dot_count > 0 && self.dot_count.is_multiple_of(80) {
            writeln!(w)?;
        }
        write!(w, "{ch}")?;
        w.flush()?;
        self.dot_count += 1;
        Ok(())
    }
}

impl Formatter for DefaultFormatter {
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

        writeln!(
            w,
            "{} passed, {} failed, {} ignored",
            result.passed, result.failed, result.ignored
        )?;

        if let Some(t) = result.exec_time {
            writeln!(w, "Finished in {t:.2}s")?;
        }

        writeln!(w)?;

        if !self.failures.is_empty() {
            writeln!(w, "Failures:\n")?;
            for (i, fail) in self.failures.iter().enumerate() {
                if let Some(t) = fail.exec_time {
                    writeln!(w, "  {}. {} ({t:.2}s)", i + 1, fail.name)?;
                } else {
                    writeln!(w, "  {}. {}", i + 1, fail.name)?;
                }
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
        }

        w.flush()?;
        Ok(())
    }
}
