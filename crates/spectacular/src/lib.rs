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
//! For those who prefer standard Rust attribute syntax — just use `#[test]`:
//!
//! ```
//! use spectacular::{test_suite, before};
//!
//! #[test_suite]
//! mod my_tests {
//!     #[before]
//!     fn setup() { }
//!
//!     #[test]
//!     fn it_works() {
//!         assert_eq!(1 + 1, 2);
//!     }
//! }
//! # fn main() {}
//! ```
//!
//! # Async Tests
//!
//! Both `spec!` and `#[test_suite]` support async test cases and hooks.
//! Specify a runtime (`tokio` or `async_std`) to enable async:
//!
//! ```
//! # // doc-test can't depend on tokio, so just show the syntax
//! # fn main() {}
//! ```
//!
//! **`spec!` style:**
//! ```ignore
//! use spectacular::spec;
//!
//! spec! {
//!     mod my_async_tests {
//!         tokio;  // or async_std;
//!
//!         async before_each { db_connect().await; }
//!
//!         async it "fetches data" {
//!             let result = fetch().await;
//!             assert!(result.is_ok());
//!         }
//!
//!         it "sync test works too" {
//!             assert_eq!(1 + 1, 2);
//!         }
//!     }
//! }
//! ```
//!
//! **Attribute style:**
//! ```ignore
//! use spectacular::{test_suite, before_each};
//!
//! #[test_suite(tokio)]
//! mod my_async_tests {
//!     #[before_each]
//!     async fn setup() { db_connect().await; }
//!
//!     #[test]
//!     async fn it_works() {
//!         let result = fetch().await;
//!         assert!(result.is_ok());
//!     }
//! }
//! ```
//!
//! Async `after_each` hooks are panic-safe — they run even if the test body
//! panics, using an async-compatible `catch_unwind` wrapper.
//!
//! **Feature-based default:** If you enable the `tokio` or `async-std` feature
//! on `spectacular`, async tests auto-detect the runtime so you can omit the
//! explicit `tokio;` / `#[test_suite(tokio)]` argument:
//!
//! ```toml
//! [dev-dependencies]
//! spectacular = { version = "0.1", features = ["tokio"] }
//! ```
//!
//! With the feature enabled, `async it` / `async fn` test cases Just Work.
//! Explicit runtime arguments always take precedence over the feature default.
//! If both features are enabled simultaneously, you must specify explicitly
//! (the macro will emit a compile error).
//!
//! # Context Injection
//!
//! Hooks can produce context values that flow naturally to tests and teardown hooks,
//! eliminating the need for `thread_local! + RefCell` patterns.
//!
//! ## `before` → shared `&T` via `OnceLock`
//!
//! When `before` returns a value, it's stored in a `OnceLock<T>`. Tests, `before_each`,
//! `after_each`, and `after` all receive `&T`.
//!
//! ## `before_each` → owned `T` per test
//!
//! When `before_each` returns a value, each test gets an owned `T`. The test borrows it
//! through `catch_unwind`, and `after_each` consumes it for cleanup.
//!
//! **How params are distinguished:** Reference params (`&T`) come from `before` context.
//! Owned params come from `before_each` context.
//!
//! **`spec!` style:**
//! ```ignore
//! use spectacular::spec;
//!
//! spec! {
//!     mod my_tests {
//!         tokio;
//!
//!         before -> PgPool {
//!             PgPool::connect("postgres://...").unwrap()
//!         }
//!
//!         after |pool: &PgPool| {
//!             pool.close();
//!         }
//!
//!         async before_each |pool: &PgPool| -> TestContext {
//!             TestContext::seed(pool).await
//!         }
//!
//!         async after_each |pool: &PgPool, ctx: TestContext| {
//!             ctx.cleanup(pool).await;
//!         }
//!
//!         async it "creates a team" |pool: &PgPool, ctx: TestContext| {
//!             // pool from before (shared &ref), ctx from before_each (owned)
//!         }
//!     }
//! }
//! ```
//!
//! **Attribute style:**
//! ```ignore
//! use spectacular::{test_suite, before, after, before_each, after_each};
//!
//! #[test_suite(tokio)]
//! mod my_tests {
//!     #[before]
//!     fn init() -> PgPool {
//!         PgPool::connect("postgres://...").unwrap()
//!     }
//!
//!     #[after]
//!     fn cleanup(pool: &PgPool) {
//!         pool.close();
//!     }
//!
//!     #[before_each]
//!     async fn setup(pool: &PgPool) -> TestContext {
//!         TestContext::seed(pool).await
//!     }
//!
//!     #[after_each]
//!     async fn teardown(pool: &PgPool, ctx: TestContext) {
//!         ctx.cleanup(pool).await;
//!     }
//!
//!     #[test]
//!     async fn test_create_team(pool: &PgPool, ctx: TestContext) {
//!         // pool from before (shared &ref), ctx from before_each (owned)
//!     }
//! }
//! ```
//!
//! Hooks without return types or params continue to work as fire-and-forget (unchanged).

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
/// For async tests, add `tokio;` or `async_std;` to the module and prefix
/// test cases or hooks with `async`: `async it "..." { ... }`,
/// `async before_each { ... }`.
///
/// Helper functions, constants, and `use` statements can appear alongside
/// hooks and test cases.
///
/// # Context Injection
///
/// Hooks can return context values using `-> Type` syntax, and receive
/// context via `|params|` syntax:
///
/// - `before -> Type { }` — returns shared context stored in `OnceLock<T>`
/// - `before_each |shared: &T| -> U { }` — receives shared `&T`, returns owned `U`
/// - `after_each |shared: &T, owned: U| { }` — receives both contexts
/// - `it "desc" |shared: &T, owned: U| { }` — receives both contexts
/// - `after |shared: &T| { }` — receives shared context
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
/// Test functions are marked with the standard `#[test]` attribute. Hook
/// functions are marked with [`#[before]`](macro@before),
/// [`#[after]`](macro@after), [`#[before_each]`](macro@before_each), or
/// [`#[after_each]`](macro@after_each).
///
/// Pass `suite` to opt into suite-level hooks: `#[test_suite(suite)]`.
///
/// For async support, pass `tokio` or `async_std`: `#[test_suite(tokio)]`.
/// Combine with suite: `#[test_suite(suite, tokio)]`. Async test and hook
/// functions are detected automatically from `async fn` signatures.
///
/// # Context Injection
///
/// Hook functions with return types or parameters enable context injection.
/// The macro reads function signatures to determine context flow:
///
/// - `#[before] fn init() -> T` — shared context via `OnceLock<T>`
/// - `#[before_each] fn setup(shared: &T) -> U` — per-test context with shared input
/// - `#[after_each] fn teardown(shared: &T, owned: U)` — receives both
/// - `#[after] fn cleanup(shared: &T)` — receives shared context
/// - `#[test] fn test_name(shared: &T, owned: U)` — receives both
///
/// Reference params (`&T`) come from `before` context. Owned params come
/// from `before_each` context.
///
/// ```
/// use spectacular::{test_suite, before_each};
///
/// #[test_suite]
/// mod my_tests {
///     #[before_each]
///     fn setup() { }
///
///     #[test]
///     fn it_works() {
///         assert_eq!(2 + 2, 4);
///     }
/// }
/// # fn main() {}
/// ```
pub use spectacular_macros::test_suite;

/// Marks a function as a once-per-group setup hook inside a
/// [`#[test_suite]`](macro@test_suite) module.
///
/// The function runs exactly once before the first test in the group. Only one
/// `#[before]` per module is allowed. Must be sync.
///
/// When the function returns a value (`fn init() -> T`), the return value is
/// stored in an `OnceLock<T>` and made available as `&T` to tests,
/// `before_each`, `after_each`, and `after` hooks via their parameters.
/// Without a return type, the hook is fire-and-forget using `Once::call_once`.
///
/// In [`spec!`] blocks, use `before { ... }` or `before -> Type { ... }`.
///
/// ```
/// use spectacular::{test_suite, before};
///
/// #[test_suite]
/// mod example {
///     #[before]
///     fn setup() { }
///
///     #[test]
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
/// Must be sync.
///
/// When `#[before]` returns context, `after` can receive it as `&T` via a
/// reference parameter: `fn cleanup(pool: &PgPool)`. Without parameters,
/// the hook is fire-and-forget.
///
/// In [`spec!`] blocks, use `after { ... }` or `after |name: &Type| { ... }`.
///
/// ```
/// use spectacular::{test_suite, after};
///
/// #[test_suite]
/// mod example {
///     #[after]
///     fn teardown() { }
///
///     #[test]
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
/// per module is allowed. Can be `async fn`.
///
/// When the function has a return type (`fn setup() -> T`), the return value
/// is passed as an owned `T` to the test and `after_each`. When the function
/// has reference parameters (`fn setup(pool: &PgPool) -> T`), those are bound
/// from the `#[before]` context. Without a return type, the hook is
/// fire-and-forget.
///
/// In [`spec!`] blocks, use `before_each { ... }` or
/// `before_each |name: &Type| -> Type { ... }`.
///
/// ```
/// use spectacular::{test_suite, before_each};
///
/// #[test_suite]
/// mod example {
///     #[before_each]
///     fn setup() { }
///
///     #[test]
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
/// per module is allowed. Can be `async fn`.
///
/// When the function has parameters, reference params (`&T`) are bound from
/// `#[before]` context, and owned params (`T`) consume the value returned by
/// `#[before_each]`. Without parameters, the hook is fire-and-forget.
///
/// In [`spec!`] blocks, use `after_each { ... }` or
/// `after_each |name: &Type, name: Type| { ... }`.
///
/// ```
/// use spectacular::{test_suite, after_each};
///
/// #[test_suite]
/// mod example {
///     #[after_each]
///     fn cleanup() { }
///
///     #[test]
///     fn my_test() {
///         assert!(true);
///     }
/// }
/// # fn main() {}
/// ```
pub use spectacular_macros::after_each;

/// Internal helpers used by generated code. Not part of the public API.
#[doc(hidden)]
pub mod __internal {
    use std::any::Any;
    use std::future::Future;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::task::Poll;

    /// Like `std::panic::catch_unwind` but for async blocks.
    ///
    /// Wraps each `poll` call in `catch_unwind` so panics inside `.await`ed
    /// futures are caught without requiring the future itself to be `UnwindSafe`.
    pub async fn catch_unwind_future<F: Future>(
        f: F,
    ) -> Result<F::Output, Box<dyn Any + Send>> {
        let mut f = Box::pin(f);
        std::future::poll_fn(move |cx| {
            match catch_unwind(AssertUnwindSafe(|| f.as_mut().poll(cx))) {
                Ok(Poll::Ready(v)) => Poll::Ready(Ok(v)),
                Ok(Poll::Pending) => Poll::Pending,
                Err(e) => Poll::Ready(Err(e)),
            }
        })
        .await
    }
}

/// Convenience re-export of all spectacular macros.
///
/// ```
/// use spectacular::prelude::*;
/// # fn main() {}
/// ```
pub mod prelude {
    pub use spectacular_macros::{
        after, after_each, before, before_each, spec, suite, test_suite,
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
