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

### `describe` syntax

You can use `describe "string"` instead of `mod name` for a more BDD-style feel. The string is automatically slugified into a valid Rust module name:

```rust
use spectacular::spec;

spec! {
    describe "basic arithmetic operations" {
        it "adds two numbers" {
            assert_eq!(2 + 2, 4);
        }
    }
}
```

This generates a module named `basic_arithmetic_operations` under the hood.

Each `spec!` invocation wraps a module. Inside the module:

- **`it "description" { body }`** -- defines a test case
- **`before { body }`** -- once-per-group setup
- **`after { body }`** -- once-per-group teardown
- **`before_each { body }`** -- per-test setup
- **`after_each { body }`** -- per-test teardown
- **`suite;`** -- opts into suite-level hooks
- Any other valid Rust items (functions, constants, `use` statements)

## Naming

Both `describe` strings and `it` descriptions are slugified into valid Rust identifiers:

| Input | Generated name |
|---|---|
| `describe "user authentication"` | `mod user_authentication` |
| `it "adds two numbers"` | `fn adds_two_numbers` |
| `it "handles UTF-8 input"` | `fn handles_utf_8_input` |
| `it "returns Ok(()) on success"` | `fn returns_ok_on_success` |

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

## Context Injection

Hooks can return context values that flow to tests and teardown hooks.

### `before -> Type`

When `before` returns a value, it's stored in an `OnceLock<T>` and available as `&T`:

```rust
use spectacular::spec;

spec! {
    mod with_context {
        before -> String {
            "shared".to_string()
        }

        it "receives shared ref" |val: &String| {
            assert_eq!(val, "shared");
        }
    }
}
```

### `before_each` with params and return type

`before_each` can receive `before`'s `&T` and return an owned per-test value:

```rust
use spectacular::spec;

spec! {
    mod full_context {
        before -> i32 { 42 }

        before_each |n: &i32| -> String {
            format!("value-{}", n)
        }

        after_each |n: &i32, s: String| {
            // n from before (&ref), s from before_each (owned)
        }

        it "gets both" |n: &i32, s: String| {
            assert_eq!(*n, 42);
            assert_eq!(s, "value-42");
        }
    }
}
```

### Pipe params on `after` and `after_each`

```rust
use spectacular::spec;

spec! {
    mod cleanup_example {
        before -> String { "resource".to_string() }

        after |r: &String| {
            // runs once, receives &String from before
        }

        before_each -> Vec<i32> { vec![1, 2, 3] }

        after_each |r: &String, data: Vec<i32>| {
            // r from before (&ref), data from before_each (owned)
        }

        it "test" |r: &String, data: Vec<i32>| {
            assert_eq!(r, "resource");
            assert_eq!(data, vec![1, 2, 3]);
        }
    }
}
```

### Context syntax summary

| Form | Description |
|------|-------------|
| `before -> Type { }` | Run-once setup returning shared context |
| `after \|name: &Type\| { }` | Run-once teardown receiving shared context |
| `before_each \|name: &Type\| -> Type { }` | Per-test setup with shared input, owned output |
| `after_each \|name: &Type, name: Type\| { }` | Per-test teardown with shared + owned context |
| `it "desc" \|name: &Type, name: Type\| { }` | Test with shared + owned context |

Hooks without return types or params continue to work as fire-and-forget (unchanged).

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
