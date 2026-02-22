//! Integration tests for feature-gated default runtime.
//!
//! Run with: `cargo test -p spectacular --features tokio`

#![cfg(all(feature = "tokio", not(feature = "async-std")))]

use spectacular::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

// ===== spec! auto-detects tokio from feature (no `tokio;` keyword) =====

async fn async_helper() -> i32 {
    tokio::task::yield_now().await;
    42
}

spec! {
    mod feature_default_spec {
        use super::*;

        async it "auto-selects tokio from feature" {
            let val = async_helper().await;
            assert_eq!(val, 42);
        }

        it "sync tests still work" {
            assert_eq!(1 + 1, 2);
        }
    }
}

// ===== #[test_suite] auto-detects tokio from feature (no explicit arg) =====

static FEATURE_ATTR_BEFORE_EACH: AtomicUsize = AtomicUsize::new(0);

#[test_suite]
mod feature_default_attr {
    use super::*;

    #[before_each]
    pub async fn setup() {
        tokio::task::yield_now().await;
        FEATURE_ATTR_BEFORE_EACH.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    pub async fn async_test_auto_runtime() {
        let val = async_helper().await;
        assert_eq!(val, 42);
        assert!(FEATURE_ATTR_BEFORE_EACH.load(Ordering::SeqCst) >= 1);
    }

    #[test]
    pub fn sync_test_in_feature_suite() {
        assert!(FEATURE_ATTR_BEFORE_EACH.load(Ordering::SeqCst) >= 1);
    }
}

// ===== Explicit runtime arg still works alongside feature =====

spec! {
    mod explicit_overrides_feature {
        use super::*;
        tokio;

        async it "explicit tokio still works" {
            let val = async_helper().await;
            assert_eq!(val, 42);
        }
    }
}

#[test_suite(tokio)]
mod explicit_attr_overrides_feature {
    use super::*;

    #[test]
    pub async fn explicit_tokio_attr() {
        let val = async_helper().await;
        assert_eq!(val, 42);
    }
}

// ===== Sync-only module with feature enabled still emits #[test] =====

spec! {
    mod sync_only_with_feature {
        it "sync test with tokio feature enabled" {
            assert_eq!(3 * 7, 21);
        }

        it "another sync test" {
            assert_eq!(1 + 1, 2);
        }
    }
}

#[test_suite]
mod sync_only_attr_with_feature {
    #[test]
    pub fn pure_sync() {
        assert_eq!(2 + 2, 4);
    }
}

// ===== spec! with async hooks, no explicit runtime =====

static FEATURE_HOOK_COUNT: AtomicUsize = AtomicUsize::new(0);

spec! {
    mod feature_default_async_hooks {
        use super::*;

        async before_each {
            tokio::task::yield_now().await;
            FEATURE_HOOK_COUNT.fetch_add(1, Ordering::SeqCst);
        }

        async it "async hooks auto-detect runtime" {
            assert!(FEATURE_HOOK_COUNT.load(Ordering::SeqCst) >= 1);
            let val = async_helper().await;
            assert_eq!(val, 42);
        }

        it "sync test with async hooks auto-detect runtime" {
            assert!(FEATURE_HOOK_COUNT.load(Ordering::SeqCst) >= 1);
        }
    }
}
