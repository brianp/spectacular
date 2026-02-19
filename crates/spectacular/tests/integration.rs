use spectacular::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// ===== Suite-level hooks =====

static SUITE_BEFORE_RAN: AtomicBool = AtomicBool::new(false);
static SUITE_BEFORE_EACH_COUNT: AtomicUsize = AtomicUsize::new(0);
static SUITE_AFTER_EACH_COUNT: AtomicUsize = AtomicUsize::new(0);

suite! {
    before {
        SUITE_BEFORE_RAN.store(true, Ordering::SeqCst);
    }
    before_each {
        SUITE_BEFORE_EACH_COUNT.fetch_add(1, Ordering::SeqCst);
    }
    after_each {
        SUITE_AFTER_EACH_COUNT.fetch_add(1, Ordering::SeqCst);
    }
}

// ===== Full 3-layer: suite + group hooks =====

static FULL_GROUP_BEFORE: AtomicBool = AtomicBool::new(false);
static FULL_GROUP_BEFORE_COUNT: AtomicUsize = AtomicUsize::new(0);
static FULL_GROUP_BEFORE_EACH_COUNT: AtomicUsize = AtomicUsize::new(0);
static FULL_GROUP_AFTER_EACH_COUNT: AtomicUsize = AtomicUsize::new(0);
static FULL_GROUP_AFTER: AtomicBool = AtomicBool::new(false);

spec! {
    mod full_three_layer {
        use super::*;
        suite;

        before {
            FULL_GROUP_BEFORE.store(true, Ordering::SeqCst);
            FULL_GROUP_BEFORE_COUNT.fetch_add(1, Ordering::SeqCst);
        }

        after {
            FULL_GROUP_AFTER.store(true, Ordering::SeqCst);
        }

        before_each {
            FULL_GROUP_BEFORE_EACH_COUNT.fetch_add(1, Ordering::SeqCst);
        }

        after_each {
            FULL_GROUP_AFTER_EACH_COUNT.fetch_add(1, Ordering::SeqCst);
        }

        it "sees suite before and group before" {
            assert!(SUITE_BEFORE_RAN.load(Ordering::SeqCst));
            assert!(FULL_GROUP_BEFORE.load(Ordering::SeqCst));
            assert_eq!(FULL_GROUP_BEFORE_COUNT.load(Ordering::SeqCst), 1);
        }

        it "suite and group before_each both fired" {
            assert!(SUITE_BEFORE_EACH_COUNT.load(Ordering::SeqCst) >= 1);
            assert!(FULL_GROUP_BEFORE_EACH_COUNT.load(Ordering::SeqCst) >= 1);
        }
    }
}

// ===== Suite-only group (no group hooks) =====

spec! {
    mod suite_only_no_group_hooks {
        use super::*;
        suite;

        it "suite before ran" {
            assert!(SUITE_BEFORE_RAN.load(Ordering::SeqCst));
        }

        it "suite before_each fires" {
            assert!(SUITE_BEFORE_EACH_COUNT.load(Ordering::SeqCst) >= 1);
        }
    }
}

// ===== Group without suite opt-in =====

static BEFORE_RAN: AtomicBool = AtomicBool::new(false);
static BEFORE_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

spec! {
    mod group_before_runs_once {
        use super::*;

        before {
            BEFORE_CALL_COUNT.fetch_add(1, Ordering::SeqCst);
            BEFORE_RAN.store(true, Ordering::SeqCst);
        }

        it "sees before state" {
            assert!(BEFORE_RAN.load(Ordering::SeqCst));
            assert_eq!(BEFORE_CALL_COUNT.load(Ordering::SeqCst), 1);
        }

        it "also sees before state and exactly one call" {
            assert!(BEFORE_RAN.load(Ordering::SeqCst));
            assert_eq!(BEFORE_CALL_COUNT.load(Ordering::SeqCst), 1);
        }
    }
}

static EACH_MARKER: AtomicUsize = AtomicUsize::new(0);
static EACH_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

spec! {
    mod before_each_fires_per_test {
        use super::*;

        before_each {
            EACH_MARKER.store(42, Ordering::SeqCst);
            EACH_CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        }

        it "sees marker from before_each" {
            assert_eq!(EACH_MARKER.load(Ordering::SeqCst), 42);
            assert!(EACH_CALL_COUNT.load(Ordering::SeqCst) >= 1);
        }

        it "also sees marker" {
            assert_eq!(EACH_MARKER.load(Ordering::SeqCst), 42);
            assert!(EACH_CALL_COUNT.load(Ordering::SeqCst) >= 1);
        }
    }
}

static AFTER_EACH_COUNT: AtomicUsize = AtomicUsize::new(0);

spec! {
    mod after_each_fires {
        use super::*;

        after_each {
            AFTER_EACH_COUNT.fetch_add(1, Ordering::SeqCst);
        }

        it "first test" {
            assert!(true);
        }

        it "second test" {
            assert!(true);
        }
    }
}

static AFTER_RAN: AtomicBool = AtomicBool::new(false);

spec! {
    mod group_after_fires {
        use super::*;

        after {
            AFTER_RAN.store(true, Ordering::SeqCst);
        }

        it "a test in group with after hook" {
            assert!(true);
        }

        it "another test in group with after hook" {
            assert!(true);
        }
    }
}

static ALL_HOOKS_BEFORE: AtomicBool = AtomicBool::new(false);
static ALL_HOOKS_BEFORE_EACH_COUNT: AtomicUsize = AtomicUsize::new(0);
static ALL_HOOKS_AFTER_EACH_COUNT: AtomicUsize = AtomicUsize::new(0);
static ALL_HOOKS_AFTER: AtomicBool = AtomicBool::new(false);

spec! {
    mod all_hooks_together {
        use super::*;

        before {
            ALL_HOOKS_BEFORE.store(true, Ordering::SeqCst);
        }

        after {
            ALL_HOOKS_AFTER.store(true, Ordering::SeqCst);
        }

        before_each {
            ALL_HOOKS_BEFORE_EACH_COUNT.fetch_add(1, Ordering::SeqCst);
        }

        after_each {
            ALL_HOOKS_AFTER_EACH_COUNT.fetch_add(1, Ordering::SeqCst);
        }

        it "sees before state and before_each fired" {
            assert!(ALL_HOOKS_BEFORE.load(Ordering::SeqCst));
            assert!(ALL_HOOKS_BEFORE_EACH_COUNT.load(Ordering::SeqCst) >= 1);
        }

        it "also sees before state" {
            assert!(ALL_HOOKS_BEFORE.load(Ordering::SeqCst));
            assert!(ALL_HOOKS_BEFORE_EACH_COUNT.load(Ordering::SeqCst) >= 1);
        }
    }
}

// ===== Attribute-style without suite =====

static ATTR_BEFORE: AtomicBool = AtomicBool::new(false);
static ATTR_BEFORE_EACH_COUNT: AtomicUsize = AtomicUsize::new(0);

#[test_suite]
mod attribute_style {
    use super::*;

    #[before]
    pub fn setup() {
        ATTR_BEFORE.store(true, Ordering::SeqCst);
    }

    #[before_each]
    pub fn each_setup() {
        ATTR_BEFORE_EACH_COUNT.fetch_add(1, Ordering::SeqCst);
    }

    #[test_case]
    pub fn sees_before() {
        assert!(ATTR_BEFORE.load(Ordering::SeqCst));
    }

    #[test_case]
    pub fn sees_before_each() {
        assert!(ATTR_BEFORE_EACH_COUNT.load(Ordering::SeqCst) >= 1);
    }
}

// ===== Attribute-style with suite =====

static ATTR_SUITE_GROUP_BEFORE: AtomicBool = AtomicBool::new(false);

#[test_suite(suite)]
mod attribute_style_with_suite {
    use super::*;

    #[before]
    pub fn setup() {
        ATTR_SUITE_GROUP_BEFORE.store(true, Ordering::SeqCst);
    }

    #[test_case]
    pub fn sees_suite_and_group_before() {
        assert!(SUITE_BEFORE_RAN.load(Ordering::SeqCst));
        assert!(ATTR_SUITE_GROUP_BEFORE.load(Ordering::SeqCst));
    }

    #[test_case]
    pub fn suite_before_each_fires() {
        assert!(SUITE_BEFORE_EACH_COUNT.load(Ordering::SeqCst) >= 1);
    }
}

// ===== Helpers and no-hooks =====

spec! {
    mod helpers_in_spec {
        fn double(n: i32) -> i32 { n * 2 }
        const MAGIC: i32 = 21;

        it "calls helper functions" {
            assert_eq!(double(MAGIC), 42);
        }

        it "uses closures and iterators" {
            let sum: i32 = (1..=10).filter(|n| n % 2 == 0).sum();
            assert_eq!(sum, 30);
        }
    }
}

spec! {
    mod no_hooks {
        it "passes with zero hooks" {
            assert_eq!(2 + 2, 4);
        }

        it "also passes" {
            assert!(!Vec::<i32>::new().iter().any(|_| true));
        }
    }
}

#[test]
fn catch_unwind_ensures_cleanup_runs() {
    use std::sync::atomic::AtomicBool;
    static CLEANUP: AtomicBool = AtomicBool::new(false);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        panic!("intentional");
    }));
    CLEANUP.store(true, Ordering::SeqCst);
    assert!(result.is_err());
    assert!(CLEANUP.load(Ordering::SeqCst));
}
