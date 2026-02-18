pub mod report;
pub mod runner;
pub mod types;

pub use inventory;

pub use spectacular_macros::{after, after_each, before, before_each, test_case, test_suite};

#[macro_export]
macro_rules! suite_before {
    ($f:expr) => {
        $crate::inventory::submit! {
            $crate::types::SuiteHook {
                kind: $crate::types::SuiteHookKind::Before,
                hook_fn: {
                    fn __spectacular_suite_before() { ($f)() }
                    __spectacular_suite_before
                },
            }
        }
    };
}

#[macro_export]
macro_rules! suite_after {
    ($f:expr) => {
        $crate::inventory::submit! {
            $crate::types::SuiteHook {
                kind: $crate::types::SuiteHookKind::After,
                hook_fn: {
                    fn __spectacular_suite_after() { ($f)() }
                    __spectacular_suite_after
                },
            }
        }
    };
}

#[macro_export]
macro_rules! suite_before_each {
    ($f:expr) => {
        $crate::inventory::submit! {
            $crate::types::SuiteHook {
                kind: $crate::types::SuiteHookKind::BeforeEach,
                hook_fn: {
                    fn __spectacular_suite_before_each() { ($f)() }
                    __spectacular_suite_before_each
                },
            }
        }
    };
}

#[macro_export]
macro_rules! suite_after_each {
    ($f:expr) => {
        $crate::inventory::submit! {
            $crate::types::SuiteHook {
                kind: $crate::types::SuiteHookKind::AfterEach,
                hook_fn: {
                    fn __spectacular_suite_after_each() { ($f)() }
                    __spectacular_suite_after_each
                },
            }
        }
    };
}

#[macro_export]
macro_rules! test_runner {
    () => {
        fn main() {
            let result = $crate::runner::run();
            std::process::exit(if result.all_passed() { 0 } else { 1 });
        }
    };
}

pub mod prelude {
    pub use crate::types::{
        SuiteHook, SuiteHookKind, SuiteResult, TestCase, TestGroup, TestOutcome, TestResult,
    };
    pub use crate::{suite_after, suite_after_each, suite_before, suite_before_each, test_runner};
    pub use spectacular_macros::{
        after, after_each, before, before_each, spec, test_case, test_suite,
    };
}
