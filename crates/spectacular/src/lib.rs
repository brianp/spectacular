pub use spectacular_macros::{
    after, after_each, before, before_each, spec, suite, test_case, test_suite,
};

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
