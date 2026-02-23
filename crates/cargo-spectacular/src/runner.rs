use crate::event::{Event, SuiteEvent, TestEvent};
use crate::formatter::Formatter;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Command, ExitCode, Stdio};

pub struct RunConfig {
    pub manifest_path: Option<String>,
    pub package: Option<String>,
    pub lib_only: bool,
    pub all: bool,
    pub extra_args: Vec<String>,
}

pub fn run(
    config: &RunConfig,
    formatter: &mut dyn Formatter,
    w: &mut dyn Write,
) -> io::Result<ExitCode> {
    let mut cmd = Command::new("cargo");
    cmd.arg("test");

    if let Some(ref path) = config.manifest_path {
        cmd.args(["--manifest-path", path]);
    }
    if let Some(ref pkg) = config.package {
        cmd.args(["--package", pkg]);
    }
    if config.lib_only {
        cmd.arg("--lib");
    }
    if config.all {
        cmd.arg("--all");
    }

    // Separator + JSON format flags + any extra user args
    cmd.arg("--");
    cmd.args(["--format", "json", "-Z", "unstable-options"]);
    cmd.args(&config.extra_args);

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::inherit());

    let mut child = cmd.spawn().map_err(|e| {
        io::Error::new(e.kind(), format!("Failed to spawn cargo test: {e}"))
    })?;

    let stdout = child.stdout.take().expect("stdout was piped");
    let reader = BufReader::new(stdout);

    let mut any_failure = false;

    for line in reader.lines() {
        let line = line?;

        let event: Event = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => continue, // skip non-JSON lines (e.g. compile output leaking through)
        };

        match event {
            Event::Suite(suite) => match suite {
                SuiteEvent::Started { test_count } => {
                    formatter.suite_started(test_count, w)?;
                }
                SuiteEvent::Ok(result) => {
                    formatter.suite_finished(&result, true, w)?;
                }
                SuiteEvent::Failed(result) => {
                    any_failure = true;
                    formatter.suite_finished(&result, false, w)?;
                }
            },
            Event::Test(test) => match test {
                TestEvent::Started { ref name } => {
                    formatter.test_started(name, w)?;
                }
                TestEvent::Ok { ref name, exec_time } => {
                    formatter.test_passed(name, exec_time, w)?;
                }
                TestEvent::Failed {
                    ref name,
                    exec_time,
                    ref stdout,
                    ref message,
                } => {
                    formatter.test_failed(
                        name,
                        exec_time,
                        stdout.as_deref(),
                        message.as_deref(),
                        w,
                    )?;
                }
                TestEvent::Ignored { ref name } => {
                    formatter.test_ignored(name, w)?;
                }
            },
        }
    }

    let status = child.wait()?;

    if any_failure || !status.success() {
        Ok(ExitCode::FAILURE)
    } else {
        Ok(ExitCode::SUCCESS)
    }
}
