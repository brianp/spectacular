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

### Attribute style

```rust
use spectacular::{test_suite, test_case};

#[test_suite]
mod arithmetic {
    #[test_case]
    fn adds_two_numbers() {
        assert_eq!(2 + 2, 4);
    }

    #[test_case]
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
    mod with_hooks {
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
    mod database_tests {
        use super::*;
        suite;

        it "has database access" {
            assert!(DB_READY.load(Ordering::SeqCst));
        }
    }
}
```

Groups **without** `suite;` skip the suite layer entirely -- no runtime cost, no coupling.

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

## Attribute Style Reference

| Attribute           | Description                                    |
|---------------------|------------------------------------------------|
| `#[test_suite]`     | Marks a module as a test group                 |
| `#[test_suite(suite)]` | Same, with suite hook opt-in                |
| `#[test_case]`      | Marks a function as a test                     |
| `#[before]`         | Once-per-group setup (max one per module)      |
| `#[after]`          | Once-per-group teardown (max one per module)   |
| `#[before_each]`    | Per-test setup (max one per module)            |
| `#[after_each]`     | Per-test teardown (max one per module)         |

## License

MIT -- see [LICENSE](LICENSE) for details.
