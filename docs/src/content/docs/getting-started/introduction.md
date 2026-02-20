---
title: Introduction
description: An RSpec-inspired test framework for Rust with stackable before/after hooks.
sidebar:
  order: 1
---

**Spectacular** is a test framework for Rust that brings RSpec-style structure to your test suites. It provides **three layers of hooks** that stack in a predictable order, giving you fine-grained control over test setup and teardown.

## Why Spectacular?

Rust's built-in `#[test]` attribute is great for simple cases, but as test suites grow, you often need:

- **Shared setup** that runs once before a group of tests
- **Per-test setup** that runs before every test in a group
- **Guaranteed cleanup** that runs even when tests panic
- **Global initialization** shared across multiple test groups

Spectacular solves all of these with a clean, layered hook system.

## Two Syntax Styles

Spectacular offers two ways to write tests:

### RSpec-style DSL

A concise, expressive syntax inspired by RSpec:

```rust
use spectacular::spec;

spec! {
    mod arithmetic {
        it "adds two numbers" {
            assert_eq!(2 + 2, 4);
        }
    }
}
```

### Attribute style

Standard Rust attributes for those who prefer a more idiomatic approach:

```rust
use spectacular::{test_suite, test_case};

#[test_suite]
mod arithmetic {
    #[test_case]
    fn adds_two_numbers() {
        assert_eq!(2 + 2, 4);
    }
}
```

Both styles support the same hook system and can be mixed within the same project.

## Three Layers of Hooks

| Layer     | Runs once per...            | Runs per test                        |
|-----------|----------------------------|--------------------------------------|
| **Suite** | binary (`before`)          | test (`before_each` / `after_each`)  |
| **Group** | group (`before` / `after`) | test (`before_each` / `after_each`)  |
| **Test**  | --                         | the test body                        |

Head to the [Installation](/spectacular/getting-started/installation/) page to get started.
