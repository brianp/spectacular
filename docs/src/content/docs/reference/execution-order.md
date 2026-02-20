---
title: Hook Execution Order
description: Detailed reference for when each hook fires.
sidebar:
  order: 1
---

## Full 3-Layer Order

When a group opts into suite hooks, each test executes hooks in this order:

```
suite::before            (Once -- first test in binary)
  group::before          (Once -- first test in group)
    suite::before_each   (every test)
      group::before_each (every test)
        TEST BODY
      group::after_each  (every test)
    suite::after_each    (every test)
  group::after           (countdown -- last test in group)
```

## Group-Only Order

When a group does **not** opt into suite hooks:

```
group::before          (Once -- first test in group)
  group::before_each   (every test)
    TEST BODY
  group::after_each    (every test)
group::after           (countdown -- last test in group)
```

## No Hooks

When a group has no hooks at all, each test runs directly with no overhead:

```
TEST BODY
```

## Detailed Behavior

### `before` (suite and group)

- Guarded by `std::sync::Once`
- Thread-safe: if multiple tests race to be "first", only one runs the hook
- The other threads block until the hook completes

### `after` (group only)

- Uses `AtomicUsize` countdown initialized to the number of tests in the group
- Each test decrements the counter after running its body and after-each hooks
- When the counter hits zero, the `after` hook fires
- Thread-safe: exactly one test triggers it

### `before_each` / `after_each`

- Called directly for every test, no synchronization needed
- Suite's hooks wrap group's hooks (suite runs first/last)

### Panic Handling

When any after hook exists (group or suite level), the test body is wrapped in `std::panic::catch_unwind`:

```rust
// Generated code (simplified)
let result = std::panic::catch_unwind(
    std::panic::AssertUnwindSafe(|| { /* test body */ })
);
// after_each hooks run here, even if test panicked
if let Err(e) = result {
    std::panic::resume_unwind(e); // re-raise after cleanup
}
```

This ensures cleanup always runs while preserving the test failure.
