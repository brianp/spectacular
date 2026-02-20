---
title: Quick Start
description: Write your first Spectacular test in under a minute.
sidebar:
  order: 3
---

## Your First Test

Create a test file and write a simple spec:

```rust
// tests/my_first_test.rs
use spectacular::spec;

spec! {
    mod arithmetic {
        it "adds two numbers" {
            assert_eq!(2 + 2, 4);
        }

        it "multiplies two numbers" {
            assert_eq!(3 * 7, 21);
        }
    }
}
```

Run it:

```bash
cargo test
```

You'll see output like:

```
running 2 tests
test arithmetic::adds_two_numbers ... ok
test arithmetic::multiplies_two_numbers ... ok
```

Each `it` block becomes a standard `#[test]` function. The description is slugified into a valid Rust identifier (`"adds two numbers"` becomes `adds_two_numbers`).

## Adding Hooks

Add a `before_each` hook to run setup before every test:

```rust
use spectacular::spec;

spec! {
    mod with_setup {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        before_each {
            COUNTER.fetch_add(1, Ordering::SeqCst);
        }

        it "counter is at least 1" {
            assert!(COUNTER.load(Ordering::SeqCst) >= 1);
        }

        it "counter keeps incrementing" {
            assert!(COUNTER.load(Ordering::SeqCst) >= 1);
        }
    }
}
```

## Prefer Attributes?

The same test using attribute style:

```rust
use spectacular::{test_suite, test_case, before_each};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test_suite]
mod with_setup {
    use super::*;

    #[before_each]
    fn setup() {
        COUNTER.fetch_add(1, Ordering::SeqCst);
    }

    #[test_case]
    fn counter_is_at_least_1() {
        assert!(COUNTER.load(Ordering::SeqCst) >= 1);
    }
}
```

## Next Steps

- [spec! DSL Guide](/spectacular/guides/spec-dsl/) -- full DSL reference
- [Attribute Style Guide](/spectacular/guides/attribute-style/) -- full attribute reference
- [Hooks Guide](/spectacular/guides/hooks/) -- all hook types explained
- [Suite Hooks](/spectacular/guides/suite-hooks/) -- the 3-layer system
