---
title: Test Runner
description: A custom cargo test runner with rainbow output, inspired by Ruby's minitest.
sidebar:
  order: 5
---

`cargo-spectacular` is a custom test runner that wraps `cargo test` and replaces the default output with dot-style progress and colored summaries. It is entirely inspired by (copied from) the fantastic work of Ryan Davis, creator of Ruby's [minitest](https://github.com/minitest/minitest) framework and its beloved pride reporter.

![Example output from cargo spectacular showing rainbow dots and a colored summary](/spectacular/test-runner.png)

## Installation

Install the runner as a cargo subcommand:

```bash
cargo install cargo-spectacular
```

Or, if building from the workspace:

```bash
cargo install --path crates/cargo-spectacular
```

## Usage

Run it like any cargo subcommand:

```bash
cargo spectacular
```

This executes `cargo test` under the hood, parses the JSON test output, and renders it with the selected formatter.

## CLI Options

```
cargo spectacular [OPTIONS] [-- <TEST_ARGS>...]
```

| Option | Description |
|--------|-------------|
| `--pride` | Rainbow dots output **(default)** |
| `--boring` | Plain dots with colored summary |
| `--manifest-path <PATH>` | Path to `Cargo.toml` |
| `--package <PKG>`, `-p <PKG>` | Run tests for a specific package |
| `--lib` | Test only the library target |
| `--all` | Test all packages in the workspace |
| `-h`, `--help` | Print help message |

Extra arguments after `--` are forwarded directly to the test binary:

```bash
cargo spectacular -- test_name          # filter tests by name
cargo spectacular -- --ignored          # run ignored tests
```

## Output Formats

### Pride (default)

The default formatter renders each test result as a rainbow-colored dot using a sine-wave color cycle -- a direct homage to minitest's `PrideLOL` reporter. Passed tests show as `.`, failures as `X`, and ignored tests as `*`.

The summary line reads "Ran N fabulous tests" in rainbow text, followed by timing and throughput stats.

### Boring

`--boring` uses plain uncolored dots for progress but still renders the summary with colored pass/fail/ignore counts. Useful when you want minimal flair but still want to see failures at a glance.

### Auto-detection

When stdout is not a terminal (e.g. piped to a file or running in CI), all color is automatically stripped and a plain-text formatter is used. No flag needed.

## Requirements

This tool requires **nightly Rust**. The `--format json` flag used internally to parse test events is unstable and requires `-Z unstable-options`, which is passed automatically.

## Attribution

The output style, rainbow coloring algorithm, and general approach are directly inspired by Ryan Davis's [minitest-pride](https://github.com/minitest/minitest) plugin for Ruby's minitest. All credit for the idea goes to him.
