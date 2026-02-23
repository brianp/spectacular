---
title: Hooks
description: Group-level before/after hooks for test setup and teardown.
sidebar:
  order: 3
---

Hooks let you run setup and teardown code around your tests. Spectacular supports four group-level hooks.

## Hook Types

### `before` -- once-per-group setup

Runs exactly once before the first test in the group. Internally guarded by `std::sync::Once`, so it's safe even when tests run in parallel.

```rust
use spectacular::spec;
use std::sync::atomic::{AtomicBool, Ordering};

static READY: AtomicBool = AtomicBool::new(false);

spec! {
    mod example {
        use super::*;

        before { READY.store(true, Ordering::SeqCst); }

        it "runs after before hook" {
            assert!(READY.load(Ordering::SeqCst));
        }
    }
}
```

### `after` -- once-per-group teardown

Runs exactly once after the last test in the group completes. Uses an atomic countdown internally -- when the last test decrements the counter to zero, the `after` hook fires.

```rust
use spectacular::spec;

spec! {
    mod example {
        after { /* cleanup shared resources */ }

        it "first test" { assert!(true); }
        it "second test" { assert!(true); }
        // after runs once, after both tests complete
    }
}
```

### `before_each` -- per-test setup

Runs before every test in the group.

```rust
use spectacular::spec;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

spec! {
    mod example {
        use super::*;

        before_each {
            COUNTER.fetch_add(1, Ordering::SeqCst);
        }

        it "counter increments" {
            assert!(COUNTER.load(Ordering::SeqCst) >= 1);
        }
    }
}
```

### `after_each` -- per-test teardown

Runs after every test in the group. Protected by `catch_unwind`, so it runs even if the test panics.

```rust
use spectacular::spec;
use std::sync::atomic::{AtomicUsize, Ordering};

static CLEANUP_COUNT: AtomicUsize = AtomicUsize::new(0);

spec! {
    mod example {
        use super::*;

        after_each {
            CLEANUP_COUNT.fetch_add(1, Ordering::SeqCst);
        }

        it "first test" { assert!(true); }
        it "second test" { assert!(true); }
    }
}
```

## Panic Safety

When `after`, `after_each`, or suite hooks are present, test bodies are wrapped in `std::panic::catch_unwind`. This ensures cleanup hooks always run, even if a test panics. After the hooks complete, the panic is re-raised so the test still reports as failed.

## Limits

- **One of each** per module: you can have at most one `before`, one `after`, one `before_each`, and one `after_each` per test group. Duplicates cause a compile error.

## Combining Hooks

All four hooks can be used together:

```rust
use spectacular::spec;

spec! {
    mod full_lifecycle {
        before      { /* once: initialize */ }
        after       { /* once: clean up */ }
        before_each { /* every test: reset state */ }
        after_each  { /* every test: verify invariants */ }

        it "test one" { assert!(true); }
        it "test two" { assert!(true); }
    }
}
```

Execution for each test:

```
before       (first test only)
  before_each
    TEST
  after_each
after        (last test only)
```

For the full 3-layer system with suite-level hooks, see [Suite Hooks](/spectacular/guides/suite-hooks/).

## Context Injection

Hooks can produce context values that flow naturally to tests and teardown hooks, eliminating the need for `thread_local!` + `RefCell` patterns.

### `before` → shared `&T` via `OnceLock`

When `before` returns a value, it's stored in an `OnceLock<T>`. Tests, `before_each`, `after_each`, and `after` all receive `&T`:

**spec! style:**

```rust
use spectacular::spec;

spec! {
    mod with_shared_context {
        before -> String {
            "shared-resource".to_string()
        }

        after |resource: &String| {
            // cleanup using the shared resource
        }

        it "receives shared ref" |resource: &String| {
            assert_eq!(resource, "shared-resource");
        }
    }
}
```

**Attribute style:**

```rust
use spectacular::{test_suite, before, after};

#[test_suite]
mod with_shared_context {
    #[before]
    fn init() -> String {
        "shared-resource".to_string()
    }

    #[after]
    fn cleanup(resource: &String) {
        // cleanup using the shared resource
    }

    #[test]
    fn receives_shared_ref(resource: &String) {
        assert_eq!(resource, "shared-resource");
    }
}
```

Without a return type, `before` works as fire-and-forget (unchanged).

### `before_each` → owned `T` per test

When `before_each` returns a value, each test gets an owned `T`. The test borrows it through `catch_unwind`, and `after_each` consumes it for cleanup:

**spec! style:**

```rust
use spectacular::spec;

spec! {
    mod with_per_test_context {
        before_each -> Vec<String> {
            vec!["initial".to_string()]
        }

        after_each |data: Vec<String>| {
            // data is consumed here for cleanup
        }

        it "receives owned value" |data: Vec<String>| {
            assert_eq!(data.len(), 1);
        }
    }
}
```

**Attribute style:**

```rust
use spectacular::{test_suite, before_each, after_each};

#[test_suite]
mod with_per_test_context {
    #[before_each]
    fn setup() -> Vec<String> {
        vec!["initial".to_string()]
    }

    #[after_each]
    fn teardown(data: Vec<String>) {
        // data is consumed here for cleanup
    }

    #[test]
    fn receives_owned_value(data: Vec<String>) {
        assert_eq!(data.len(), 1);
    }
}
```

Without a return type, `before_each` works as fire-and-forget (unchanged).

### Full stack: `before` + `before_each`

`before_each` can receive the shared `&T` from `before` to produce per-test context:

**spec! style:**

```rust
use spectacular::spec;

spec! {
    mod full_context {
        before -> i32 {
            42
        }

        before_each |shared: &i32| -> String {
            format!("ctx-{}", shared)
        }

        after_each |shared: &i32, owned: String| {
            // shared is &i32 from before, owned is String from before_each
        }

        it "gets both" |shared: &i32, owned: String| {
            assert_eq!(*shared, 42);
            assert_eq!(owned, "ctx-42");
        }
    }
}
```

**Attribute style:**

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

### Inferred return type (`-> _`)

When `before_each` returns a type that can't be named explicitly (e.g. `impl Trait`), use `-> _` to let the compiler infer it. The macro inlines the body at each call site instead of generating a function:

**spec! style:**

```rust
use spectacular::spec;

spec! {
    mod inferred_return {
        before_each -> _ {
            (String::from("hello"), 42u32)
        }

        after_each |s: _, n: _| {
            // types inferred from before_each
        }

        it "receives inferred values" |s: _, n: _| {
            assert_eq!(s, "hello");
            assert_eq!(n, 42);
        }
    }
}
```

**Attribute style:**

```rust
use spectacular::{test_suite, before_each, after_each};

#[test_suite]
mod inferred_return {
    #[before_each]
    fn setup() -> _ {
        (String::from("hello"), 42u32)
    }

    #[after_each]
    fn teardown(s: _, n: _) {
        // types inferred from before_each
    }

    #[test]
    fn receives_inferred(s: _, n: _) {
        assert_eq!(s, "hello");
        assert_eq!(n, 42);
    }
}
```

`-> _` is **not** supported on `before` because it stores context in `OnceLock<T>`, which requires a concrete type. The macro emits a compile error if you try.

### How params are distinguished

The macro distinguishes context sources by type:

- **Reference params (`&T`)** come from `before` context (shared, read-only)
- **Owned params (`T`)** come from `before_each` context (per-test, consumed by `after_each`)
- **Inferred params (`_`)** also come from `before_each` context, with the type inferred by the compiler
