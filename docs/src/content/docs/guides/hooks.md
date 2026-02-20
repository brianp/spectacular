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
