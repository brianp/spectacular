//! An RSpec-inspired test framework for Rust with stackable before/after hooks.
//!
//! Spectacular provides three layers of test hooks that stack in a predictable order:
//!
//! | Layer | Runs once per… | Runs per test |
//! |-------|----------------|---------------|
//! | **Suite** | binary (`before`) | test (`before_each` / `after_each`) |
//! | **Group** | group (`before` / `after`) | test (`before_each` / `after_each`) |
//! | **Test** | — | the test body |
//!
//! # Hook Execution Order
//!
//! For each test in a group that opts into suite hooks:
//!
//! ```text
//! suite::before          (Once — first test in binary triggers it)
//!   group::before        (Once — first test in group triggers it)
//!     suite::before_each
//!       group::before_each
//!         TEST
//!       group::after_each
//!     suite::after_each
//!   group::after         (countdown — last test in group triggers it)
//! ```
//!
//! Groups without `suite;` skip the suite layer entirely.
//!
//! # Quick Start
//!
//! ```
//! use spectacular::spec;
//!
//! spec! {
//!     mod arithmetic {
//!         it "adds two numbers" {
//!             assert_eq!(2 + 2, 4);
//!         }
//!
//!         it "multiplies two numbers" {
//!             assert_eq!(3 * 7, 21);
//!         }
//!     }
//! }
//! # fn main() {}
//! ```
//!
//! # Group Hooks
//!
//! ```
//! use spectacular::spec;
//! use std::sync::atomic::{AtomicBool, Ordering};
//!
//! static READY: AtomicBool = AtomicBool::new(false);
//!
//! spec! {
//!     mod with_hooks {
//!         use super::*;
//!
//!         before { READY.store(true, Ordering::SeqCst); }
//!
//!         it "runs after setup" {
//!             assert!(READY.load(Ordering::SeqCst));
//!         }
//!     }
//! }
//! # fn main() {}
//! ```
//!
//! # Suite Hooks (3-Layer)
//!
//! Place [`suite!`] as a sibling of your test groups, then opt in with `suite;`
//! (in [`spec!`]) or `#[test_suite(suite)]` (attribute style):
//!
//! ```
//! use spectacular::{suite, spec};
//! use std::sync::atomic::{AtomicBool, Ordering};
//!
//! static DB_READY: AtomicBool = AtomicBool::new(false);
//!
//! suite! {
//!     before { DB_READY.store(true, Ordering::SeqCst); }
//! }
//!
//! spec! {
//!     mod database_tests {
//!         use super::*;
//!         suite;
//!
//!         it "has database access" {
//!             assert!(DB_READY.load(Ordering::SeqCst));
//!         }
//!     }
//! }
//! # fn main() {}
//! ```
//!
//! # Attribute Style
//!
//! For those who prefer standard Rust attribute syntax:
//!
//! ```
//! use spectacular::{test_suite, test_case, before};
//!
//! #[test_suite]
//! mod my_tests {
//!     #[before]
//!     fn setup() { }
//!
//!     #[test_case]
//!     fn it_works() {
//!         assert_eq!(1 + 1, 2);
//!     }
//! }
//! # fn main() {}
//! ```

/// Defines suite-level hooks that run across all opted-in test groups.
///
/// Generates a hidden `__spectacular_suite` module containing `before()`,
/// `before_each()`, and `after_each()` functions. The `before` hook uses
/// [`std::sync::Once`] internally so it executes at most once per test binary.
///
/// Groups opt in with `suite;` inside [`spec!`] or `#[test_suite(suite)]` for
/// attribute style. Groups without opt-in are completely unaffected.
///
/// All three hook types are optional. Omitted hooks generate empty functions.
///
/// ```
/// use spectacular::{suite, spec};
///
/// suite! {
///     before { /* runs once per binary */ }
///     before_each { /* runs before each opted-in test */ }
///     after_each { /* runs after each opted-in test */ }
/// }
///
/// spec! {
///     mod my_group {
///         suite;
///
///         it "uses suite hooks" {
///             assert!(true);
///         }
///     }
/// }
/// # fn main() {}
/// ```
pub use spectacular_macros::suite;

/// Defines a test group using RSpec-style DSL.
///
/// Each `it "description" { body }` block becomes a `#[test]` function whose
/// name is the slugified description. Groups support `before`, `after`,
/// `before_each`, and `after_each` hooks. Add `suite;` to opt into
/// suite-level hooks defined by [`suite!`].
///
/// Helper functions, constants, and `use` statements can appear alongside
/// hooks and test cases.
///
/// ```
/// use spectacular::spec;
///
/// spec! {
///     mod my_group {
///         fn helper() -> i32 { 42 }
///
///         before_each { /* per-test setup */ }
///
///         it "uses a helper" {
///             assert_eq!(helper(), 42);
///         }
///
///         it "does arithmetic" {
///             assert_eq!(2 + 2, 4);
///         }
///     }
/// }
/// # fn main() {}
/// ```
pub use spectacular_macros::spec;

/// Marks a module as a test suite using standard Rust attribute syntax.
///
/// Functions annotated with [`#[test_case]`](macro@test_case) become `#[test]`
/// functions. Hook functions are marked with [`#[before]`](macro@before),
/// [`#[after]`](macro@after), [`#[before_each]`](macro@before_each), or
/// [`#[after_each]`](macro@after_each).
///
/// Pass `suite` to opt into suite-level hooks: `#[test_suite(suite)]`.
///
/// ```
/// use spectacular::{test_suite, test_case, before_each};
///
/// #[test_suite]
/// mod my_tests {
///     #[before_each]
///     fn setup() { }
///
///     #[test_case]
///     fn it_works() {
///         assert_eq!(2 + 2, 4);
///     }
/// }
/// # fn main() {}
/// ```
pub use spectacular_macros::test_suite;

/// Marks a function as a test case inside a [`#[test_suite]`](macro@test_suite) module.
///
/// The function is transformed into a `#[test]` function with any configured
/// hooks applied around the body.
///
/// ```
/// use spectacular::{test_suite, test_case};
///
/// #[test_suite]
/// mod example {
///     #[test_case]
///     fn two_plus_two() {
///         assert_eq!(2 + 2, 4);
///     }
/// }
/// # fn main() {}
/// ```
pub use spectacular_macros::test_case;

/// Marks a function as a once-per-group setup hook inside a
/// [`#[test_suite]`](macro@test_suite) module.
///
/// The function runs exactly once before the first test in the group, guarded
/// by [`std::sync::Once`]. Only one `#[before]` per module is allowed.
///
/// In [`spec!`] blocks, use `before { ... }` instead.
///
/// ```
/// use spectacular::{test_suite, test_case, before};
///
/// #[test_suite]
/// mod example {
///     #[before]
///     fn setup() { }
///
///     #[test_case]
///     fn my_test() {
///         assert!(true);
///     }
/// }
/// # fn main() {}
/// ```
pub use spectacular_macros::before;

/// Marks a function as a once-per-group teardown hook inside a
/// [`#[test_suite]`](macro@test_suite) module.
///
/// The function runs exactly once after the last test in the group completes,
/// using an atomic countdown. Only one `#[after]` per module is allowed.
///
/// In [`spec!`] blocks, use `after { ... }` instead.
///
/// ```
/// use spectacular::{test_suite, test_case, after};
///
/// #[test_suite]
/// mod example {
///     #[after]
///     fn teardown() { }
///
///     #[test_case]
///     fn my_test() {
///         assert!(true);
///     }
/// }
/// # fn main() {}
/// ```
pub use spectacular_macros::after;

/// Marks a function as a per-test setup hook inside a
/// [`#[test_suite]`](macro@test_suite) module.
///
/// The function runs before every test in the group. Only one `#[before_each]`
/// per module is allowed.
///
/// In [`spec!`] blocks, use `before_each { ... }` instead.
///
/// ```
/// use spectacular::{test_suite, test_case, before_each};
///
/// #[test_suite]
/// mod example {
///     #[before_each]
///     fn setup() { }
///
///     #[test_case]
///     fn my_test() {
///         assert!(true);
///     }
/// }
/// # fn main() {}
/// ```
pub use spectacular_macros::before_each;

/// Marks a function as a per-test teardown hook inside a
/// [`#[test_suite]`](macro@test_suite) module.
///
/// The function runs after every test in the group, even if the test panics
/// (protected by [`std::panic::catch_unwind`]). Only one `#[after_each]`
/// per module is allowed.
///
/// In [`spec!`] blocks, use `after_each { ... }` instead.
///
/// ```
/// use spectacular::{test_suite, test_case, after_each};
///
/// #[test_suite]
/// mod example {
///     #[after_each]
///     fn cleanup() { }
///
///     #[test_case]
///     fn my_test() {
///         assert!(true);
///     }
/// }
/// # fn main() {}
/// ```
pub use spectacular_macros::after_each;

/// Convenience re-export of all spectacular macros.
///
/// ```
/// use spectacular::prelude::*;
/// # fn main() {}
/// ```
pub mod prelude {
    pub use spectacular_macros::{
        after, after_each, before, before_each, spec, suite, test_case, test_suite,
    };
}

#[cfg(test)]
mod tests {
    use super::prelude::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    static UNIT_SUITE_BEFORE: AtomicBool = AtomicBool::new(false);
    static UNIT_SUITE_BEFORE_EACH: AtomicUsize = AtomicUsize::new(0);
    static UNIT_SUITE_AFTER_EACH: AtomicUsize = AtomicUsize::new(0);

    suite! {
        before {
            UNIT_SUITE_BEFORE.store(true, Ordering::SeqCst);
        }
        before_each {
            UNIT_SUITE_BEFORE_EACH.fetch_add(1, Ordering::SeqCst);
        }
        after_each {
            UNIT_SUITE_AFTER_EACH.fetch_add(1, Ordering::SeqCst);
        }
    }

    spec! {
        mod suite_in_unit_tests {
            use super::*;
            suite;

            it "suite hooks work in unit test context" {
                assert!(UNIT_SUITE_BEFORE.load(Ordering::SeqCst));
                assert!(UNIT_SUITE_BEFORE_EACH.load(Ordering::SeqCst) >= 1);
            }

            it "suite before_each fires for each test" {
                assert!(UNIT_SUITE_BEFORE_EACH.load(Ordering::SeqCst) >= 1);
            }
        }
    }

    spec! {
        mod group_without_suite_in_unit {
            it "works without suite opt-in" {
                assert_eq!(2 + 2, 4);
            }
        }
    }
}
