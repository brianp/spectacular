use spectacular::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

static AFTER_EACH_FIRED: AtomicBool = AtomicBool::new(false);
static GROUP_AFTER_FIRED: AtomicBool = AtomicBool::new(false);
static SIBLING_TEST_RAN: AtomicBool = AtomicBool::new(false);

spec! {
    mod panic_resilience {
        use super::*;

        after {
            GROUP_AFTER_FIRED.store(true, Ordering::SeqCst);
        }

        after_each {
            AFTER_EACH_FIRED.store(true, Ordering::SeqCst);
        }

        it "deliberately panics" {
            panic!("this panic is intentional — testing failure handling");
        }

        it "runs despite a sibling test panicking" {
            SIBLING_TEST_RAN.store(true, Ordering::SeqCst);
            assert!(true);
        }
    }
}

fn main() {
    let result = spectacular::runner::run();

    assert!(
        AFTER_EACH_FIRED.load(Ordering::SeqCst),
        "after_each must fire even when a test panics"
    );

    assert!(
        GROUP_AFTER_FIRED.load(Ordering::SeqCst),
        "group after hook must fire even when a test in the group panicked"
    );

    assert!(
        SIBLING_TEST_RAN.load(Ordering::SeqCst),
        "sibling test must still run even when another test in the group panicked"
    );

    assert_eq!(result.passed(), 1, "exactly one test should pass");
    assert_eq!(result.failed(), 1, "exactly one test should fail");
    assert!(!result.all_passed(), "suite should NOT report all_passed");

    eprintln!("\n=== failure_handling post-run assertions: ALL PASSED ===");
    std::process::exit(0);
}
