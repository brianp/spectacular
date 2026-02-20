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
    mod group_name {
        // hooks and tests go here
    }
}
```

**Keywords:**

| Keyword | Usage | Description |
|---------|-------|-------------|
| `it` | `it "desc" { body }` | Defines a test case |
| `before` | `before { body }` | Once-per-group setup |
| `after` | `after { body }` | Once-per-group teardown |
| `before_each` | `before_each { body }` | Per-test setup |
| `after_each` | `after_each { body }` | Per-test teardown |
| `suite;` | `suite;` | Opt into suite hooks |

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
#[test_suite]        // without suite hooks
#[test_suite(suite)] // with suite hook opt-in
mod my_tests { }
```

### `#[test_case]`

Marks a function as a test case. Must be inside a `#[test_suite]` module.

```rust
#[test_case]
fn my_test() { }
```

### `#[before]`

Marks a function as a once-per-group setup hook. Max one per module.

```rust
#[before]
fn setup() { }
```

### `#[after]`

Marks a function as a once-per-group teardown hook. Max one per module.

```rust
#[after]
fn teardown() { }
```

### `#[before_each]`

Marks a function as a per-test setup hook. Max one per module.

```rust
#[before_each]
fn setup() { }
```

### `#[after_each]`

Marks a function as a per-test teardown hook. Max one per module.

```rust
#[after_each]
fn cleanup() { }
```

## Prelude

Import everything at once:

```rust
use spectacular::prelude::*;
```

This re-exports: `spec`, `suite`, `test_suite`, `test_case`, `before`, `after`, `before_each`, `after_each`.

## Full API Documentation

For the complete generated documentation with doc-tests, see [docs.rs/spectacular](https://docs.rs/spectacular).
