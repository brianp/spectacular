---
title: Installation
description: How to add Spectacular to your Rust project.
sidebar:
  order: 2
---

Add `spectacular` to your `Cargo.toml` as a dev dependency:

```toml
[dev-dependencies]
spectacular = "0.1"
```

Or use `cargo add`:

```bash
cargo add --dev spectacular
```

That's it. Spectacular is a proc-macro crate with zero runtime dependencies beyond `std`. It works with the standard `cargo test` runner -- no custom test harness required.

## Minimum Rust Version

Spectacular requires **Rust 2024 edition** (1.85+).

## Crate Structure

The `spectacular` crate is a thin facade that re-exports macros from `spectacular-macros`. You only need to depend on `spectacular` directly -- the proc-macro crate is pulled in automatically.

```
spectacular            # facade crate (add this to Cargo.toml)
  └── spectacular-macros   # proc-macro implementation (automatic)
```

## Next Steps

Head to the [Quick Start](/spectacular/getting-started/quick-start/) to write your first test.
