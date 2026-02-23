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

## Context Injection

Hook functions can return context values and receive context via parameters. The macro reads function signatures to determine context flow.

### `#[before]` returning context

When the `#[before]` function has a return type, the value is stored in an `OnceLock<T>` and available as `&T` to all other hooks and tests:

```rust
use spectacular::{test_suite, before};

#[test_suite]
mod with_shared_context {
    #[before]
    fn init() -> String {
        "shared".to_string()
    }

    #[test]
    fn receives_ref(val: &String) {
        assert_eq!(val, "shared");
    }
}
```

### `#[before_each]` returning context

When `#[before_each]` has a return type, each test gets an owned value. Reference params on `before_each` are bound from `before`'s context:

```rust
use spectacular::{test_suite, before, before_each, after_each};

#[test_suite]
mod full_context {
    #[before]
    fn init() -> i32 { 42 }

    #[before_each]
    fn setup(shared: &i32) -> String {
        format!("ctx-{}", shared)
    }

    #[after_each]
    fn teardown(shared: &i32, owned: String) {
        // shared is &i32 from before, owned is String from before_each
    }

    #[test]
    fn gets_both(shared: &i32, owned: String) {
        assert_eq!(*shared, 42);
        assert_eq!(owned, "ctx-42");
    }
}
```

### How params are distinguished

The macro distinguishes context sources by type:

- **Reference params (`&T`)** → bound from `#[before]` context (shared, read-only)
- **Owned params (`T`)** → bound from `#[before_each]` context (per-test)

### Inferred context (no return type)

Hooks can omit their return type and let the macro infer everything from downstream consumers.

#### `#[before]` — inferred from `&T` params

When `#[before]` has no return type but a downstream hook or test uses an explicit `&T` param, the macro infers `OnceLock<T>` automatically:

```rust
use spectacular::{test_suite, before};

#[test_suite]
mod inferred_before {
    #[before]
    fn init() {
        42i32
    }

    #[test]
    fn receives_ref(val: &i32) {
        assert_eq!(*val, 42);
    }
}
```

#### `#[before_each]` — inferred from `_` params

When `#[before_each]` has no return type, the last expression of the body **is** the context. Tests use `_` as the param type:

```rust
use spectacular::{test_suite, before_each};

#[test_suite]
mod inferred_context {
    #[before_each]
    fn setup() {
        (String::from("hello"), 42u32)
    }

    #[test]
    fn receives_inferred(s: _, n: _) {
        assert_eq!(s, "hello");
        assert_eq!(n, 42);
    }
}
```

The macro detects `_`-typed params in tests or `#[after_each]` and automatically inlines the `#[before_each]` body. Without `_` params or `&T` consumers, hooks with no return type are fire-and-forget as usual.

### Context reference

| Pattern | Description |
|---------|-------------|
| `fn init() -> T` | `#[before]` returning shared context (explicit) |
| `fn init()` | `#[before]` with inferred context (when consumers use `&T` params) |
| `fn cleanup(x: &T)` | `#[after]` receiving shared context |
| `fn setup(x: &T) -> U` | `#[before_each]` with shared input, owned output |
| `fn setup()` | `#[before_each]` with inferred context (when tests use `_` params) |
| `fn teardown(x: &T, y: U)` | `#[after_each]` with shared + owned |
| `fn teardown(x: &T, y: _)` | `#[after_each]` with inferred owned type |
| `fn test_name(x: &T, y: U)` | `#[test]` with shared + owned |

Hooks without return types or `_` params continue to work as fire-and-forget (unchanged).
