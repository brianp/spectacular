---
title: API Reference
description: Complete reference for all Spectacular macros.
sidebar:
  order: 2
---

## Macros

### `spec!`

Defines a test group using RSpec-style DSL.

```rust
use spectacular::spec;

spec! {
    describe "group name" {
        // hooks and tests go here
    }
}
```

You can also use `mod name` directly instead of `describe "string"`.

**Keywords:**

| Keyword | Usage | Description |
|---------|-------|-------------|
| `describe` | `describe "name" { }` | BDD-style group (string slugified to module name) |
| `mod` | `mod name { }` | Group with explicit module name |
| `it` | `it "desc" { body }` | Defines a test case |
| `it` | `it "desc" \|params\| { body }` | Test with context params |
| `before` | `before { body }` | Once-per-group setup (fire-and-forget) |
| `before` | `before -> Type { body }` | Once-per-group setup returning shared context |
| `after` | `after { body }` | Once-per-group teardown |
| `after` | `after \|params\| { body }` | Teardown receiving shared context |
| `before_each` | `before_each { body }` | Per-test setup (fire-and-forget) |
| `before_each` | `before_each \|params\| -> Type { body }` | Per-test setup with context |
| `after_each` | `after_each { body }` | Per-test teardown |
| `after_each` | `after_each \|params\| { body }` | Teardown receiving context |
| `suite;` | `suite;` | Opt into suite hooks |
| `tokio;` | `tokio;` | Use tokio async runtime |
| `async_std;` | `async_std;` | Use async-std async runtime |

Pipe params use `|name: Type, name: Type|` syntax. Reference params (`&T`) bind from `before` context; owned params bind from `before_each` context.

### `suite!`

Defines suite-level hooks that apply across all opted-in groups.

```rust
use spectacular::suite;

suite! {
    before      { /* once per binary */ }
    before_each { /* before each opted-in test */ }
    after_each  { /* after each opted-in test */ }
}
```

All three blocks are optional. Generates a hidden `__spectacular_suite` module.

## Attributes

### `#[test_suite]`

Marks a module as a test group.

```rust
#[test_suite]              // without suite hooks
#[test_suite(suite)]       // with suite hook opt-in
#[test_suite(tokio)]       // with tokio async runtime
#[test_suite(suite, tokio)] // both
mod my_tests { }
```

### `#[before]`

Marks a function as a once-per-group setup hook. Max one per module. Must be sync.

```rust
#[before]
fn setup() { }            // fire-and-forget

#[before]
fn setup() -> PgPool { }  // returns shared context (OnceLock<PgPool>)
```

When returning a value, other hooks and tests receive `&T` via reference parameters.

### `#[after]`

Marks a function as a once-per-group teardown hook. Max one per module. Must be sync.

```rust
#[after]
fn teardown() { }                  // fire-and-forget

#[after]
fn teardown(pool: &PgPool) { }    // receives shared context from #[before]
```

### `#[before_each]`

Marks a function as a per-test setup hook. Max one per module. Can be `async fn`.

```rust
#[before_each]
fn setup() { }                               // fire-and-forget

#[before_each]
fn setup() -> TestContext { }                 // returns per-test context

#[before_each]
fn setup(pool: &PgPool) -> TestContext { }   // receives shared, returns per-test
```

Reference params bind from `#[before]` context. Return value is passed as owned to tests and `#[after_each]`.

### `#[after_each]`

Marks a function as a per-test teardown hook. Max one per module. Can be `async fn`.

```rust
#[after_each]
fn cleanup() { }                              // fire-and-forget

#[after_each]
fn cleanup(pool: &PgPool, ctx: TestContext) { } // shared + owned context
```

Reference params bind from `#[before]`, owned params consume the `#[before_each]` return value.

## Prelude

Import everything at once:

```rust
use spectacular::prelude::*;
```

This re-exports: `spec`, `suite`, `test_suite`, `before`, `after`, `before_each`, `after_each`.

## Full API Documentation

For the complete generated documentation with doc-tests, see [docs.rs/spectacular](https://docs.rs/spectacular).
