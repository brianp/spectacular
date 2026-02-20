---
title: "spec! DSL"
description: Write tests using Spectacular's RSpec-inspired DSL.
sidebar:
  order: 1
---

The `spec!` macro provides a concise, RSpec-inspired syntax for writing test groups.

## Basic Structure

```rust
use spectacular::spec;

spec! {
    mod group_name {
        it "description of test" {
            // test body
        }
    }
}
```

Each `spec!` invocation wraps a module. Inside the module:

- **`it "description" { body }`** -- defines a test case
- **`before { body }`** -- once-per-group setup
- **`after { body }`** -- once-per-group teardown
- **`before_each { body }`** -- per-test setup
- **`after_each { body }`** -- per-test teardown
- **`suite;`** -- opts into suite-level hooks
- Any other valid Rust items (functions, constants, `use` statements)

## Test Naming

Test descriptions are slugified into valid Rust identifiers:

| Description | Function name |
|---|---|
| `"adds two numbers"` | `adds_two_numbers` |
| `"handles UTF-8 input"` | `handles_utf_8_input` |
| `"returns Ok(()) on success"` | `returns_ok_on_success` |

## Helper Functions

You can define helper functions and constants alongside tests:

```rust
use spectacular::spec;

spec! {
    mod helpers {
        fn double(n: i32) -> i32 { n * 2 }
        const MAGIC: i32 = 21;

        it "uses helpers" {
            assert_eq!(double(MAGIC), 42);
        }
    }
}
```

## Imports

Use `use super::*;` or specific imports to bring items from the enclosing scope:

```rust
use spectacular::spec;
use std::collections::HashMap;

fn make_map() -> HashMap<&'static str, i32> {
    HashMap::from([("a", 1), ("b", 2)])
}

spec! {
    mod with_imports {
        use super::*;

        it "uses imported function" {
            let map = make_map();
            assert_eq!(map["a"], 1);
        }
    }
}
```

## All Hooks Together

```rust
use spectacular::spec;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static INITIALIZED: AtomicBool = AtomicBool::new(false);
static CLEANED_UP: AtomicBool = AtomicBool::new(false);
static SETUP_COUNT: AtomicUsize = AtomicUsize::new(0);
static TEARDOWN_COUNT: AtomicUsize = AtomicUsize::new(0);

spec! {
    mod full_example {
        use super::*;

        before {
            INITIALIZED.store(true, Ordering::SeqCst);
        }

        after {
            CLEANED_UP.store(true, Ordering::SeqCst);
        }

        before_each {
            SETUP_COUNT.fetch_add(1, Ordering::SeqCst);
        }

        after_each {
            TEARDOWN_COUNT.fetch_add(1, Ordering::SeqCst);
        }

        it "sees initialized state" {
            assert!(INITIALIZED.load(Ordering::SeqCst));
        }

        it "setup runs per test" {
            assert!(SETUP_COUNT.load(Ordering::SeqCst) >= 1);
        }
    }
}
```

## Visibility

The generated module inherits the visibility you declare:

```rust
use spectacular::spec;

spec! {
    pub mod public_tests {
        it "is accessible from other modules" {
            assert!(true);
        }
    }
}
```
