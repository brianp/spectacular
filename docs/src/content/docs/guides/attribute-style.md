---
title: Attribute Style
description: Write tests using standard Rust attributes.
sidebar:
  order: 2
---

For developers who prefer idiomatic Rust syntax, Spectacular provides an attribute-based API that mirrors the `spec!` DSL.

## Basic Structure

```rust
use spectacular::test_suite;

#[test_suite]
mod my_tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn it_also_works() {
        assert!(true);
    }
}
```

`#[test_suite]` marks the module. Tests use the standard `#[test]` attribute. Each `#[test]` function gets hooks applied automatically.

## Attributes Reference

| Attribute              | Description                                |
|------------------------|--------------------------------------------|
| `#[test_suite]`        | Marks a module as a test group             |
| `#[test_suite(suite)]` | Same, with suite hook opt-in               |
| `#[test]`              | Marks a function as a test                 |
| `#[before]`            | Once-per-group setup (max one per module)  |
| `#[after]`             | Once-per-group teardown (max one per module) |
| `#[before_each]`       | Per-test setup (max one per module)        |
| `#[after_each]`        | Per-test teardown (max one per module)     |

## Adding Hooks

```rust
use spectacular::{test_suite, before, after, before_each, after_each};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static INITIALIZED: AtomicBool = AtomicBool::new(false);
static SETUP_COUNT: AtomicUsize = AtomicUsize::new(0);

#[test_suite]
mod with_hooks {
    use super::*;

    #[before]
    fn initialize() {
        INITIALIZED.store(true, Ordering::SeqCst);
    }

    #[before_each]
    fn setup() {
        SETUP_COUNT.fetch_add(1, Ordering::SeqCst);
    }

    #[after_each]
    fn teardown() {
        // runs after every test, even on panic
    }

    #[after]
    fn cleanup() {
        // runs once after the last test
    }

    #[test]
    fn sees_initialization() {
        assert!(INITIALIZED.load(Ordering::SeqCst));
    }

    #[test]
    fn setup_runs_per_test() {
        assert!(SETUP_COUNT.load(Ordering::SeqCst) >= 1);
    }
}
```

## Helper Functions

Non-annotated functions are passed through as-is:

```rust
use spectacular::test_suite;

#[test_suite]
mod with_helpers {
    fn double(n: i32) -> i32 { n * 2 }

    #[test]
    fn uses_helper() {
        assert_eq!(double(21), 42);
    }
}
```

## Suite Opt-in

Pass `suite` to opt into suite-level hooks:

```rust
use spectacular::{suite, test_suite, before};
use std::sync::atomic::{AtomicBool, Ordering};

static DB_READY: AtomicBool = AtomicBool::new(false);

suite! {
    before { DB_READY.store(true, Ordering::SeqCst); }
}

#[test_suite(suite)]
mod database_tests {
    use super::*;

    #[before]
    fn group_setup() {
        // runs after suite::before, once per group
    }

    #[test]
    fn has_database() {
        assert!(DB_READY.load(Ordering::SeqCst));
    }
}
```

See [Suite Hooks](/spectacular/guides/suite-hooks/) for the full 3-layer system.
