---
title: Suite Hooks
description: The 3-layer hook system for shared setup across test groups.
sidebar:
  order: 4
---

Suite hooks add a third layer above group hooks. They let you share expensive setup (database connections, test servers, fixture loading) across multiple test groups.

## The Three Layers

| Layer     | Runs once per...            | Runs per test                        |
|-----------|----------------------------|--------------------------------------|
| **Suite** | binary (`before`)          | test (`before_each` / `after_each`)  |
| **Group** | group (`before` / `after`) | test (`before_each` / `after_each`)  |
| **Test**  | --                         | the test body                        |

## Defining Suite Hooks

Place `suite!` at the module level, as a sibling of your test groups:

```rust
use spectacular::{suite, spec};
use std::sync::atomic::{AtomicBool, Ordering};

static DB_READY: AtomicBool = AtomicBool::new(false);

suite! {
    before      { DB_READY.store(true, Ordering::SeqCst); }
    before_each { /* begin transaction */ }
    after_each  { /* rollback transaction */ }
}
```

All three suite hook types are optional. Omitted hooks generate empty functions.

The `suite!` macro generates a hidden `__spectacular_suite` module with well-known function names. The `before` body is wrapped in `std::sync::Once`, so it runs at most once per test binary regardless of how many groups opt in.

## Opting In

Groups must explicitly opt into suite hooks. Groups without opt-in are completely unaffected.

### DSL style

Add `suite;` inside the `spec!` block:

```rust
spec! {
    mod database_tests {
        use super::*;
        suite;

        it "has database access" {
            assert!(DB_READY.load(Ordering::SeqCst));
        }
    }
}
```

### Attribute style

Pass `suite` to the `#[test_suite]` attribute:

```rust
use spectacular::test_suite;

#[test_suite(suite)]
mod database_tests {
    use super::*;

    #[test]
    fn has_database_access() {
        assert!(DB_READY.load(Ordering::SeqCst));
    }
}
```

## Execution Order

For each test in a group that opts into suite hooks:

```
suite::before            (Once -- first test in binary triggers it)
  group::before          (Once -- first test in group triggers it)
    suite::before_each
      group::before_each
        TEST
      group::after_each
    suite::after_each
  group::after           (countdown -- last test in group triggers it)
```

Key details:

- **Suite before** runs at most once per binary, guarded by `std::sync::Once`
- **Group before** runs at most once per group, also guarded by `Once`
- **Suite before_each** runs before group's `before_each`, for every test
- **After hooks** run in reverse order (innermost first)
- **Group after** uses an atomic countdown -- the last test in the group triggers it

## Mixing Opted-in and Standalone Groups

```rust
use spectacular::{suite, spec};
use std::sync::atomic::{AtomicBool, Ordering};

static EXPENSIVE_RESOURCE: AtomicBool = AtomicBool::new(false);

suite! {
    before { EXPENSIVE_RESOURCE.store(true, Ordering::SeqCst); }
}

// This group uses suite hooks
spec! {
    mod needs_resource {
        use super::*;
        suite;

        it "has the resource" {
            assert!(EXPENSIVE_RESOURCE.load(Ordering::SeqCst));
        }
    }
}

// This group does NOT use suite hooks -- zero overhead
spec! {
    mod standalone {
        it "works independently" {
            assert_eq!(2 + 2, 4);
        }
    }
}
```

## Practical Example: Database Testing

A common use case is wrapping each test in a database transaction:

```rust
use spectacular::{suite, spec};

suite! {
    before {
        // Run migrations, seed test data
        // This happens once per `cargo test` invocation
    }
    before_each {
        // Begin a savepoint/transaction
    }
    after_each {
        // Rollback the transaction
        // Each test sees a clean database state
    }
}

spec! {
    mod user_tests {
        use super::*;
        suite;

        it "creates a user" {
            // test runs inside a transaction that gets rolled back
        }

        it "updates a user" {
            // also runs in its own rolled-back transaction
        }
    }
}

spec! {
    mod order_tests {
        use super::*;
        suite;

        it "creates an order" {
            // same suite hooks, different group hooks possible
        }
    }
}
```

### With Context Injection

The database example becomes much cleaner with context injection. Instead of `thread_local!` + `RefCell`, hooks return values directly:

**spec! style:**

```rust
use spectacular::spec;

spec! {
    mod database_tests {
        tokio;

        before -> PgPool {
            PgPool::connect("postgres://localhost/test").unwrap()
        }

        after |pool: &PgPool| {
            pool.close();
        }

        async before_each |pool: &PgPool| -> Transaction {
            pool.begin().await.unwrap()
        }

        async after_each |_pool: &PgPool, tx: Transaction| {
            tx.rollback().await.unwrap();
        }

        async it "creates a user" |pool: &PgPool, tx: Transaction| {
            // pool is &PgPool (shared), tx is Transaction (owned per test)
        }
    }
}
```

**Attribute style:**

```rust
use spectacular::{test_suite, before, after, before_each, after_each};

#[test_suite(tokio)]
mod database_tests {
    #[before]
    fn init() -> PgPool {
        PgPool::connect("postgres://localhost/test").unwrap()
    }

    #[after]
    fn cleanup(pool: &PgPool) {
        pool.close();
    }

    #[before_each]
    async fn begin_tx(pool: &PgPool) -> Transaction {
        pool.begin().await.unwrap()
    }

    #[after_each]
    async fn rollback(pool: &PgPool, tx: Transaction) {
        tx.rollback().await.unwrap();
    }

    #[test]
    async fn creates_a_user(pool: &PgPool, tx: Transaction) {
        // pool is &PgPool (shared), tx is Transaction (owned per test)
    }
}
```
