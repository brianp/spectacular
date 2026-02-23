mod event;
mod formatter;
mod runner;

use runner::RunConfig;
use std::io::IsTerminal;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    let mut format_name = String::from("pride");
    let mut manifest_path: Option<String> = None;
    let mut package: Option<String> = None;
    let mut lib_only = false;
    let mut all = false;
    let mut extra_args: Vec<String> = Vec::new();

    let iter = args.iter().skip(1); // skip binary name
    // Skip "spectacular" if invoked as `cargo spectacular`
    let mut args_to_parse: Vec<&String> = Vec::new();
    let mut skipped_subcommand = false;
    for arg in iter {
        if !skipped_subcommand && arg == "spectacular" {
            skipped_subcommand = true;
            continue;
        }
        args_to_parse.push(arg);
    }

    let mut i = 0;
    let mut after_separator = false;
    while i < args_to_parse.len() {
        let arg = args_to_parse[i].as_str();

        if after_separator {
            extra_args.push(arg.to_string());
            i += 1;
            continue;
        }

        match arg {
            "--" => {
                after_separator = true;
            }
            "--pride" => {
                format_name = String::from("pride");
            }
            "--boring" => {
                format_name = String::from("boring");
            }
            "--manifest-path" => {
                i += 1;
                if i < args_to_parse.len() {
                    manifest_path = Some(args_to_parse[i].clone());
                } else {
                    eprintln!("Error: --manifest-path requires a value");
                    return ExitCode::FAILURE;
                }
            }
            "--package" | "-p" => {
                i += 1;
                if i < args_to_parse.len() {
                    package = Some(args_to_parse[i].clone());
                } else {
                    eprintln!("Error: --package requires a value");
                    return ExitCode::FAILURE;
                }
            }
            "--lib" => {
                lib_only = true;
            }
            "--all" => {
                all = true;
            }
            "--help" | "-h" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            other => {
                // Treat unknown args as extra test args
                extra_args.push(other.to_string());
            }
        }
        i += 1;
    }

    // Auto-downgrade to no-color when stdout isn't a terminal
    if !std::io::stdout().is_terminal() {
        format_name = String::from("default");
    }

    let mut fmt = formatter::create(&format_name);
    let mut stdout = std::io::stdout().lock();

    let config = RunConfig {
        manifest_path,
        package,
        lib_only,
        all,
        extra_args,
    };

    match runner::run(&config, fmt.as_mut(), &mut stdout) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!(
        "\
cargo-spectacular — custom test output formatter

USAGE:
    cargo spectacular [OPTIONS] [-- <TEST_ARGS>...]

OPTIONS:
    --pride                   Rainbow dots output (default)
    --boring                  Plain dots, colored summary
    --manifest-path <PATH>    Path to Cargo.toml
    --package, -p <PKG>       Run tests for a specific package
    --lib                     Test only the library
    --all                     Test all packages in the workspace
    -h, --help                Print this help message

ARGS:
    <TEST_ARGS>...            Extra arguments passed to the test binary

NOTE:
    This tool requires nightly Rust. The --format json flag used internally
    is unstable and requires -Z unstable-options.

    When stdout is not a terminal, all color is automatically stripped.

EXAMPLES:
    cargo spectacular                          # pride (default)
    cargo spectacular --boring                 # plain dots, colored summary
    cargo spectacular -- test_name             # filter tests
    cargo spectacular --package my-crate       # specific package"
    );
}
