<p align="center">
  <img src="spectacular.png" alt="Spectacular" width="280">
</p>

<h3 align="center">An RSpec-inspired test framework for Rust</h3>

<p align="center">
  Stackable before/after hooks with three layers of control.
</p>

<p align="center">
  <a href="https://crates.io/crates/spectacular"><img src="https://img.shields.io/crates/v/spectacular.svg" alt="crates.io"></a>
  <a href="https://docs.rs/spectacular"><img src="https://docs.rs/spectacular/badge.svg" alt="docs.rs"></a>
  <a href="LICENSE"><img src="https://img.shields.io/crates/l/spectacular.svg" alt="MIT License"></a>
</p>

---

Spectacular gives your Rust tests **three layers of hooks** that stack in a predictable order:

| Layer     | Runs once per...  | Runs per test                        |
|-----------|-------------------|--------------------------------------|
| **Suite** | binary (`before`) | test (`before_each` / `after_each`)  |
| **Group** | group (`before` / `after`) | test (`before_each` / `after_each`) |
| **Test**  | --                | the test body                        |

## Installation

```toml
[dev-dependencies]
spectacular = "0.1"
```

## Quick Start

### RSpec-style DSL

```rust
use spectacular::spec;

spec! {
    describe "arithmetic" {
        it "adds two numbers" {
            assert_eq!(2 + 2, 4);
        }

        it "multiplies two numbers" {
            assert_eq!(3 * 7, 21);
        }
    }
}
```

`describe "string"` slugifies the string into a module name (`"arithmetic"` → `mod arithmetic`). You can also use `mod name` directly if you prefer.

### Attribute style

```rust
use spectacular::test_suite;

#[test_suite]
mod arithmetic {
    #[test]
    fn adds_two_numbers() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn multiplies_two_numbers() {
        assert_eq!(3 * 7, 21);
    }
}
```

## Hooks

### Group hooks

Group hooks run within a single test module. `before` runs once before the first test; `after` runs once after the last test. `before_each` and `after_each` run around every test.

```rust
use spectacular::spec;
use std::sync::atomic::{AtomicBool, Ordering};

static READY: AtomicBool = AtomicBool::new(false);

spec! {
    describe "with hooks" {
        use super::*;

        before { READY.store(true, Ordering::SeqCst); }
        after  { /* cleanup */ }
        before_each { /* per-test setup */ }
        after_each  { /* per-test teardown */ }

        it "runs after setup" {
            assert!(READY.load(Ordering::SeqCst));
        }
    }
}
```

### Suite hooks (3-layer)

Suite hooks run across **all** opted-in groups. Place `suite!` as a sibling of your test groups, then opt in with `suite;` (DSL) or `#[test_suite(suite)]` (attribute style):

```rust
use spectacular::{suite, spec};
use std::sync::atomic::{AtomicBool, Ordering};

static DB_READY: AtomicBool = AtomicBool::new(false);

suite! {
    before      { DB_READY.store(true, Ordering::SeqCst); }
    before_each { /* per-test transaction */ }
    after_each  { /* rollback */ }
}

spec! {
    describe "database tests" {
        use super::*;
        suite;

        it "has database access" {
            assert!(DB_READY.load(Ordering::SeqCst));
        }
    }
}
```

Groups **without** `suite;` skip the suite layer entirely -- no runtime cost, no coupling.

## Context Injection

Hooks can produce context values that flow naturally to tests and teardown hooks, eliminating `thread_local! + RefCell` patterns.

- **`before` → shared `&T`**: When `before` returns a value, it's stored in an `OnceLock<T>`. Tests, `before_each`, `after_each`, and `after` all receive `&T`.
- **`before_each` → owned `T`**: When `before_each` returns a value, each test gets an owned `T`. The test borrows it through `catch_unwind`, and `after_each` consumes it for cleanup.

**How params are distinguished:** Reference params (`&T`) come from `before` context. Owned params come from `before_each` context.

### RSpec-style DSL

```rust
use spectacular::spec;

spec! {
    describe "database tests" {
        tokio;

        before -> PgPool {
            PgPool::connect("postgres://...").unwrap()
        }

        after |pool: &PgPool| {
            pool.close();
        }

        async before_each |pool: &PgPool| -> TestContext {
            TestContext::seed(pool).await
        }

        async after_each |pool: &PgPool, ctx: TestContext| {
            ctx.cleanup(pool).await;
        }

        async it "creates a team" |pool: &PgPool, ctx: TestContext| {
            // pool from before (shared &ref), ctx from before_each (owned)
        }
    }
}
```

### Attribute style

```rust
use spectacular::{test_suite, before, after, before_each, after_each};

#[test_suite(tokio)]
mod database_tests {
    #[before]
    fn init() -> PgPool {
        PgPool::connect("postgres://...").unwrap()
    }

    #[after]
    fn cleanup(pool: &PgPool) {
        pool.close();
    }

    #[before_each]
    async fn setup(pool: &PgPool) -> TestContext {
        TestContext::seed(pool).await
    }

    #[after_each]
    async fn teardown(pool: &PgPool, ctx: TestContext) {
        ctx.cleanup(pool).await;
    }

    #[test]
    async fn test_create_team(pool: &PgPool, ctx: TestContext) {
        // pool from before (shared &ref), ctx from before_each (owned)
    }
}
```

Hooks without return types or params continue to work as fire-and-forget (unchanged).

## Hook Execution Order

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

After-hooks are protected by `catch_unwind`, so cleanup runs even if a test panics.

## Async Tests

Both `spec!` and `#[test_suite]` support async test cases and hooks. Specify a runtime (`tokio` or `async_std`) to enable async:

```rust
use spectacular::spec;

spec! {
    describe "my async tests" {
        tokio;  // or async_std;

        async before_each { db_connect().await; }

        async it "fetches data" {
            let result = fetch().await;
            assert!(result.is_ok());
        }

        it "sync test works too" {
            assert_eq!(1 + 1, 2);
        }
    }
}
```

**Feature-based default:** Enable the `tokio` or `async-std` feature on `spectacular` to auto-detect the runtime:

```toml
[dev-dependencies]
spectacular = { version = "0.1", features = ["tokio"] }
```

With the feature enabled, `async it` / `async fn` test cases Just Work without explicit `tokio;` or `#[test_suite(tokio)]`.

## Attribute Style Reference

| Attribute           | Description                                    |
|---------------------|------------------------------------------------|
| `#[test_suite]`     | Marks a module as a test group                 |
| `#[test_suite(suite)]` | Same, with suite hook opt-in                |
| `#[test_suite(tokio)]` | Async test group with tokio runtime         |
| `#[test]`           | Marks a function as a test                     |
| `#[before]`         | Once-per-group setup (max one per module)      |
| `#[after]`          | Once-per-group teardown (max one per module)   |
| `#[before_each]`    | Per-test setup (max one per module)            |
| `#[after_each]`     | Per-test teardown (max one per module)         |

## Context Injection Reference

### spec! syntax

| Form | Description |
|------|-------------|
| `describe "name" { }` | BDD-style group (string slugified to module name) |
| `mod name { }` | Group with explicit module name |
| `before -> Type { }` | Run-once setup returning shared context |
| `after \|name: &Type\| { }` | Run-once teardown receiving shared context |
| `before_each \|name: &Type\| -> Type { }` | Per-test setup with shared context input, owned output |
| `after_each \|name: &Type, name: Type\| { }` | Per-test teardown with shared + owned context |
| `it "desc" \|name: &Type, name: Type\| { }` | Test with shared + owned context |

### Attribute syntax

| Pattern | Description |
|---------|-------------|
| `fn init() -> T` | `#[before]` returning shared context |
| `fn cleanup(x: &T)` | `#[after]` receiving shared context |
| `fn setup(x: &T) -> U` | `#[before_each]` with shared input, owned output |
| `fn teardown(x: &T, y: U)` | `#[after_each]` with shared + owned |
| `fn test_name(x: &T, y: U)` | `#[test]` with shared + owned |

## License

MIT -- see [LICENSE](LICENSE) for details.
