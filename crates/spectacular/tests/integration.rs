use spectacular::prelude::*;
use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};

thread_local! {
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

fn log(msg: &str) {
    LOG.with(|l| l.borrow_mut().push(msg.to_string()));
}

fn log_snapshot() -> Vec<String> {
    LOG.with(|l| l.borrow().clone())
}

fn assert_order(earlier: &str, later: &str) {
    let entries = log_snapshot();
    let pos_earlier = entries.iter().position(|e| e == earlier);
    let pos_later = entries.iter().position(|e| e == later);
    match (pos_earlier, pos_later) {
        (Some(a), Some(b)) => {
            assert!(
                a < b,
                "expected '{}' (pos {}) before '{}' (pos {})\nfull log: {:?}",
                earlier,
                a,
                later,
                b,
                entries
            );
        }
        (None, _) => panic!(
            "'{}' never appeared in log.\nfull log: {:?}",
            earlier, entries
        ),
        (_, None) => panic!(
            "'{}' never appeared in log.\nfull log: {:?}",
            later, entries
        ),
    }
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

suite_before!(|| {
    log("suite:before");
});

suite_after!(|| {
    log("suite:after");
});

suite_before_each!(|| {
    log("suite:before_each");
});

suite_after_each!(|| {
    log("suite:after_each");
});

spec! {
    mod hook_ordering {
        use super::*;

        before {
            log("ordering:before");
        }

        after {
            log("ordering:after");
        }

        before_each {
            log("ordering:before_each");
        }

        after_each {
            log("ordering:after_each");
        }

        it "suite:before ran before group:before" {
            assert_order("suite:before", "ordering:before");
        }

        it "group:before ran before group:before_each" {
            assert_order("ordering:before", "ordering:before_each");
        }

        it "suite:before_each ran before group:before_each" {
            let entries = log_snapshot();
            let last_suite_be = entries.iter().rposition(|e| e == "suite:before_each").unwrap();
            let last_group_be = entries.iter().rposition(|e| e == "ordering:before_each").unwrap();
            assert!(
                last_suite_be < last_group_be,
                "suite:before_each (pos {}) should fire before ordering:before_each (pos {})\nlog: {:?}",
                last_suite_be, last_group_be, entries
            );
        }

        it "group:after_each ran before suite:after_each within this group" {
            let entries = log_snapshot();
            let group_start = entries.iter().position(|e| e == "ordering:before").unwrap();
            let group_entries: Vec<&String> = entries[group_start..].iter().collect();

            let local_gae = group_entries.iter().position(|e| e.as_str() == "ordering:after_each");
            if let Some(gae_pos) = local_gae {
                let local_sae = group_entries[gae_pos..].iter()
                    .position(|e| e.as_str() == "suite:after_each");
                assert!(
                    local_sae.is_some(),
                    "suite:after_each should follow ordering:after_each within this group\ngroup log: {:?}",
                    group_entries
                );
            }
        }
    }
}

spec! {
    mod before_each_isolation {
        use super::*;

        before {
            COUNTER.store(999, Ordering::SeqCst);
        }

        before_each {
            COUNTER.store(0, Ordering::SeqCst);
        }

        after_each {
            COUNTER.fetch_add(50, Ordering::SeqCst);
        }

        it "starts at 0 from before_each" {
            assert_eq!(
                COUNTER.load(Ordering::SeqCst), 0,
                "before_each should have reset counter to 0"
            );
            COUNTER.store(42, Ordering::SeqCst);
        }

        it "does not see prior test's mutations" {
            assert_eq!(
                COUNTER.load(Ordering::SeqCst), 0,
                "before_each must reset counter regardless of prior test"
            );
        }

        it "does not see after_each side effects" {
            assert_eq!(
                COUNTER.load(Ordering::SeqCst), 0,
                "before_each should overwrite after_each's mutation"
            );
        }
    }
}

#[test_suite]
mod attribute_style {
    use super::*;

    #[before]
    pub fn setup() {
        log("attr:before");
    }

    #[after]
    pub fn teardown() {
        log("attr:after");
    }

    #[before_each]
    pub fn each_setup() {
        log("attr:before_each");
    }

    #[after_each]
    pub fn each_teardown() {
        log("attr:after_each");
    }

    #[test_case]
    pub fn suite_before_precedes_group_before() {
        assert_order("suite:before", "attr:before");
    }

    #[test_case]
    pub fn group_before_precedes_before_each() {
        assert_order("attr:before", "attr:before_each");
    }

    #[test_case]
    pub fn suite_before_each_precedes_group_before_each() {
        let entries = log_snapshot();
        let last_sbe = entries
            .iter()
            .rposition(|e| e == "suite:before_each")
            .unwrap();
        let last_gbe = entries
            .iter()
            .rposition(|e| e == "attr:before_each")
            .unwrap();
        assert!(
            last_sbe < last_gbe,
            "suite:before_each ({}) should precede attr:before_each ({})",
            last_sbe,
            last_gbe
        );
    }
}

spec! {
    mod spec_with_helpers {
        fn double(n: i32) -> i32 { n * 2 }
        const MAGIC: i32 = 21;

        it "calls helper functions defined in the same block" {
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
        it "passes with zero hooks defined" {
            assert_eq!(2 + 2, 4);
        }

        it "also passes" {
            assert!(!Vec::<i32>::new().iter().any(|_| true));
        }
    }
}

test_runner!();
