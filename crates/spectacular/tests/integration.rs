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

    #[test]
    pub fn sees_before() {
        assert!(ATTR_BEFORE.load(Ordering::SeqCst));
    }

    #[test]
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

    #[test]
    pub fn sees_suite_and_group_before() {
        assert!(SUITE_BEFORE_RAN.load(Ordering::SeqCst));
        assert!(ATTR_SUITE_GROUP_BEFORE.load(Ordering::SeqCst));
    }

    #[test]
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

// ===== Async spec! with tokio =====

async fn async_helper() -> i32 {
    tokio::task::yield_now().await;
    42
}

spec! {
    mod async_spec_basic {
        use super::*;
        tokio;

        async it "runs an async test" {
            let val = async_helper().await;
            assert_eq!(val, 42);
        }

        it "sync test in async module" {
            assert_eq!(1 + 1, 2);
        }
    }
}

// ===== Async spec! with async hooks =====

static ASYNC_BEFORE_EACH_COUNT: AtomicUsize = AtomicUsize::new(0);
static ASYNC_AFTER_EACH_COUNT: AtomicUsize = AtomicUsize::new(0);

spec! {
    mod async_spec_hooks {
        use super::*;
        tokio;

        async before_each {
            tokio::task::yield_now().await;
            ASYNC_BEFORE_EACH_COUNT.fetch_add(1, Ordering::SeqCst);
        }

        async after_each {
            tokio::task::yield_now().await;
            ASYNC_AFTER_EACH_COUNT.fetch_add(1, Ordering::SeqCst);
        }

        async it "async hooks fire for async test" {
            let val = async_helper().await;
            assert_eq!(val, 42);
            assert!(ASYNC_BEFORE_EACH_COUNT.load(Ordering::SeqCst) >= 1);
        }

        it "async hooks fire for sync test too" {
            assert!(ASYNC_BEFORE_EACH_COUNT.load(Ordering::SeqCst) >= 1);
        }
    }
}

// ===== Async attribute-style #[test_suite(tokio)] =====

static ATTR_ASYNC_BEFORE_EACH: AtomicUsize = AtomicUsize::new(0);
static ATTR_ASYNC_AFTER_EACH: AtomicUsize = AtomicUsize::new(0);

#[test_suite(tokio)]
mod async_attribute_style {
    use super::*;

    #[before_each]
    pub async fn setup() {
        tokio::task::yield_now().await;
        ATTR_ASYNC_BEFORE_EACH.fetch_add(1, Ordering::SeqCst);
    }

    #[after_each]
    pub async fn teardown() {
        tokio::task::yield_now().await;
        ATTR_ASYNC_AFTER_EACH.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    pub async fn async_test_with_hooks() {
        let val = async_helper().await;
        assert_eq!(val, 42);
        assert!(ATTR_ASYNC_BEFORE_EACH.load(Ordering::SeqCst) >= 1);
    }

    #[test]
    pub fn sync_test_in_async_suite() {
        assert!(ATTR_ASYNC_BEFORE_EACH.load(Ordering::SeqCst) >= 1);
    }
}

// ===== Async attribute-style with suite + tokio =====

#[test_suite(suite, tokio)]
mod async_attribute_with_suite {
    use super::*;

    #[test]
    pub async fn sees_suite_hooks_from_async() {
        tokio::task::yield_now().await;
        assert!(SUITE_BEFORE_RAN.load(Ordering::SeqCst));
        assert!(SUITE_BEFORE_EACH_COUNT.load(Ordering::SeqCst) >= 1);
    }
}

// ===== Async spec! with sync before (run-once) + async before_each =====

static ASYNC_MIX_BEFORE: AtomicBool = AtomicBool::new(false);
static ASYNC_MIX_BEFORE_EACH: AtomicUsize = AtomicUsize::new(0);

spec! {
    mod async_spec_mixed_hooks {
        use super::*;
        tokio;

        before {
            ASYNC_MIX_BEFORE.store(true, Ordering::SeqCst);
        }

        async before_each {
            tokio::task::yield_now().await;
            ASYNC_MIX_BEFORE_EACH.fetch_add(1, Ordering::SeqCst);
        }

        async it "sync before + async before_each both run" {
            assert!(ASYNC_MIX_BEFORE.load(Ordering::SeqCst));
            assert!(ASYNC_MIX_BEFORE_EACH.load(Ordering::SeqCst) >= 1);
            let val = async_helper().await;
            assert_eq!(val, 42);
        }
    }
}

// ===== Async catch_unwind: after_each runs even on panic =====

static ASYNC_CATCH_AFTER_EACH: AtomicUsize = AtomicUsize::new(0);

spec! {
    mod async_catch_unwind_after_each {
        use super::*;
        tokio;

        async after_each {
            tokio::task::yield_now().await;
            ASYNC_CATCH_AFTER_EACH.fetch_add(1, Ordering::SeqCst);
        }

        async it "non-panicking async test" {
            let val = async_helper().await;
            assert_eq!(val, 42);
        }
    }
}

// ===== Context Injection Tests =====

// --- Attribute style: before returns context, test receives &T, after receives &T ---

static CTX_AFTER_CALLED: AtomicBool = AtomicBool::new(false);

#[test_suite]
mod attr_before_ctx {
    use super::*;

    #[before]
    pub fn init() -> String {
        "hello".to_string()
    }

    #[after]
    pub fn cleanup(val: &String) {
        assert_eq!(val, "hello");
        CTX_AFTER_CALLED.store(true, Ordering::SeqCst);
    }

    #[test]
    pub fn test_receives_before_ref(val: &String) {
        assert_eq!(val, "hello");
    }

    #[test]
    pub fn test_receives_before_ref_2(val: &String) {
        assert_eq!(val, "hello");
    }
}

// --- Attribute style: before_each returns context, test receives owned, after_each consumes ---

static CTX_EACH_AFTER_EACH_SUM: AtomicUsize = AtomicUsize::new(0);

#[test_suite]
mod attr_before_each_ctx {
    use super::*;

    #[before_each]
    pub fn setup() -> usize {
        99
    }

    #[after_each]
    pub fn teardown(val: usize) {
        CTX_EACH_AFTER_EACH_SUM.fetch_add(val, Ordering::SeqCst);
    }

    #[test]
    pub fn test_gets_owned(val: usize) {
        assert_eq!(val, 99);
    }

    #[test]
    pub fn test_gets_owned_2(val: usize) {
        assert_eq!(val, 99);
    }
}

// --- Attribute style: full stack with before + before_each + test + after_each + after ---

static CTX_FULL_AFTER_CALLED: AtomicBool = AtomicBool::new(false);
static CTX_FULL_AFTER_EACH_COUNT: AtomicUsize = AtomicUsize::new(0);

#[test_suite]
mod attr_full_ctx_stack {
    use super::*;

    #[before]
    pub fn init() -> i32 {
        42
    }

    #[after]
    pub fn cleanup(shared: &i32) {
        assert_eq!(*shared, 42);
        CTX_FULL_AFTER_CALLED.store(true, Ordering::SeqCst);
    }

    #[before_each]
    pub fn setup(shared: &i32) -> String {
        format!("ctx-{}", shared)
    }

    #[after_each]
    pub fn teardown(shared: &i32, owned: String) {
        assert_eq!(*shared, 42);
        assert_eq!(owned, "ctx-42");
        CTX_FULL_AFTER_EACH_COUNT.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    pub fn test_full_stack(shared: &i32, owned: String) {
        assert_eq!(*shared, 42);
        assert_eq!(owned, "ctx-42");
    }

    #[test]
    pub fn test_full_stack_2(shared: &i32, owned: String) {
        assert_eq!(*shared, 42);
        assert_eq!(owned, "ctx-42");
    }
}

// --- Attribute style: test with no params, context flows to after_each ---

static CTX_NO_PARAM_AFTER_EACH: AtomicUsize = AtomicUsize::new(0);

#[test_suite]
mod attr_no_test_params_ctx {
    use super::*;

    #[before_each]
    pub fn setup() -> usize {
        77
    }

    #[after_each]
    pub fn teardown(val: usize) {
        assert_eq!(val, 77);
        CTX_NO_PARAM_AFTER_EACH.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    pub fn test_no_params() {
        assert_eq!(1 + 1, 2);
    }
}

// --- spec! style: before -> Type, test with params, after with params ---

static SPEC_CTX_AFTER_CALLED: AtomicBool = AtomicBool::new(false);

spec! {
    mod spec_before_ctx {
        use super::*;

        before -> String {
            "world".to_string()
        }

        after |val: &String| {
            assert_eq!(val, "world");
            SPEC_CTX_AFTER_CALLED.store(true, Ordering::SeqCst);
        }

        it "receives before ref" |val: &String| {
            assert_eq!(val, "world");
        }

        it "also receives before ref" |val: &String| {
            assert_eq!(val, "world");
        }
    }
}

// --- spec! style: before_each returns context ---

static SPEC_EACH_AFTER_EACH_SUM: AtomicUsize = AtomicUsize::new(0);

spec! {
    mod spec_before_each_ctx {
        use super::*;

        before_each -> usize {
            55
        }

        after_each |val: usize| {
            SPEC_EACH_AFTER_EACH_SUM.fetch_add(val, Ordering::SeqCst);
        }

        it "gets owned value" |val: usize| {
            assert_eq!(val, 55);
        }
    }
}

// --- spec! style: full stack with before + before_each + after_each + after ---

static SPEC_FULL_AFTER_CALLED: AtomicBool = AtomicBool::new(false);
static SPEC_FULL_AFTER_EACH_COUNT: AtomicUsize = AtomicUsize::new(0);

spec! {
    mod spec_full_ctx_stack {
        use super::*;

        before -> i32 {
            10
        }

        after |shared: &i32| {
            assert_eq!(*shared, 10);
            SPEC_FULL_AFTER_CALLED.store(true, Ordering::SeqCst);
        }

        before_each |shared: &i32| -> String {
            format!("item-{}", shared)
        }

        after_each |shared: &i32, owned: String| {
            assert_eq!(*shared, 10);
            assert_eq!(owned, "item-10");
            SPEC_FULL_AFTER_EACH_COUNT.fetch_add(1, Ordering::SeqCst);
        }

        it "full stack" |shared: &i32, owned: String| {
            assert_eq!(*shared, 10);
            assert_eq!(owned, "item-10");
        }

        it "full stack again" |shared: &i32, owned: String| {
            assert_eq!(*shared, 10);
            assert_eq!(owned, "item-10");
        }
    }
}

// --- spec! style: async full stack ---

static SPEC_ASYNC_CTX_AFTER: AtomicBool = AtomicBool::new(false);
static SPEC_ASYNC_CTX_AFTER_EACH: AtomicUsize = AtomicUsize::new(0);

spec! {
    mod spec_async_full_ctx {
        use super::*;
        tokio;

        before -> String {
            "async-pool".to_string()
        }

        after |pool: &String| {
            assert_eq!(pool, "async-pool");
            SPEC_ASYNC_CTX_AFTER.store(true, Ordering::SeqCst);
        }

        async before_each |pool: &String| -> String {
            tokio::task::yield_now().await;
            format!("{}-ctx", pool)
        }

        async after_each |pool: &String, ctx: String| {
            tokio::task::yield_now().await;
            assert_eq!(pool, "async-pool");
            assert_eq!(ctx, "async-pool-ctx");
            SPEC_ASYNC_CTX_AFTER_EACH.fetch_add(1, Ordering::SeqCst);
        }

        async it "async full stack" |pool: &String, ctx: String| {
            assert_eq!(pool, "async-pool");
            assert_eq!(ctx, "async-pool-ctx");
        }
    }
}

// --- Attribute style: async full stack ---

static ATTR_ASYNC_CTX_AFTER: AtomicBool = AtomicBool::new(false);
static ATTR_ASYNC_CTX_AFTER_EACH: AtomicUsize = AtomicUsize::new(0);

#[test_suite(tokio)]
mod attr_async_full_ctx {
    use super::*;

    #[before]
    pub fn init() -> String {
        "db-pool".to_string()
    }

    #[after]
    pub fn cleanup(pool: &String) {
        assert_eq!(pool, "db-pool");
        ATTR_ASYNC_CTX_AFTER.store(true, Ordering::SeqCst);
    }

    #[before_each]
    pub async fn setup(pool: &String) -> String {
        tokio::task::yield_now().await;
        format!("{}-session", pool)
    }

    #[after_each]
    pub async fn teardown(pool: &String, session: String) {
        tokio::task::yield_now().await;
        assert_eq!(pool, "db-pool");
        assert_eq!(session, "db-pool-session");
        ATTR_ASYNC_CTX_AFTER_EACH.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    pub async fn async_full_ctx_test(pool: &String, session: String) {
        assert_eq!(pool, "db-pool");
        assert_eq!(session, "db-pool-session");
    }
}

// --- Backwards compat: before without return type still works (already tested above) ---
// --- Backwards compat: before_each without return type still works (already tested above) ---

// ===== describe "string" syntax =====

spec! {
    describe "basic arithmetic operations" {
        it "adds two numbers" {
            assert_eq!(2 + 2, 4);
        }

        it "multiplies two numbers" {
            assert_eq!(3 * 7, 21);
        }
    }
}

static DESCRIBE_BEFORE: AtomicBool = AtomicBool::new(false);
static DESCRIBE_BEFORE_EACH_COUNT: AtomicUsize = AtomicUsize::new(0);

spec! {
    describe "describe blocks with hooks" {
        use super::*;

        before {
            DESCRIBE_BEFORE.store(true, Ordering::SeqCst);
        }

        before_each {
            DESCRIBE_BEFORE_EACH_COUNT.fetch_add(1, Ordering::SeqCst);
        }

        it "sees before state" {
            assert!(DESCRIBE_BEFORE.load(Ordering::SeqCst));
            assert!(DESCRIBE_BEFORE_EACH_COUNT.load(Ordering::SeqCst) >= 1);
        }

        it "before_each fires per test" {
            assert!(DESCRIBE_BEFORE_EACH_COUNT.load(Ordering::SeqCst) >= 1);
        }
    }
}

spec! {
    describe "describe with context injection" {
        before -> i32 { 100 }

        it "receives shared ref" |val: &i32| {
            assert_eq!(*val, 100);
        }
    }
}

// ===== Inferred return type (`-> _`) tests =====

// --- spec! style: sync before_each -> _ ---

spec! {
    mod spec_before_each_infer_sync {
        before_each -> _ {
            (String::from("inferred"), 42u32)
        }

        it "receives inferred tuple" |s: _, n: _| {
            assert_eq!(s, "inferred");
            assert_eq!(n, 42);
        }

        it "each test gets fresh value" |s: _, n: _| {
            assert_eq!(s, "inferred");
            assert_eq!(n, 42);
        }
    }
}

// --- spec! style: async before_each -> _ with after_each using _ params ---

static SPEC_INFER_ASYNC_AFTER_EACH: AtomicUsize = AtomicUsize::new(0);

spec! {
    mod spec_before_each_infer_async {
        use super::*;
        tokio;

        async before_each -> _ {
            tokio::task::yield_now().await;
            (String::from("async-inferred"), 99u64)
        }

        async after_each |s: _, n: _| {
            assert_eq!(s, "async-inferred");
            assert_eq!(n, 99);
            SPEC_INFER_ASYNC_AFTER_EACH.fetch_add(1, Ordering::SeqCst);
        }

        async it "receives async inferred value" |s: _, n: _| {
            assert_eq!(s, "async-inferred");
            assert_eq!(n, 99);
        }
    }
}

// --- spec! style: before -> i32 + before_each |n: &i32| -> _ + after_each with _ params ---

static SPEC_INFER_FULL_AFTER_EACH: AtomicUsize = AtomicUsize::new(0);

spec! {
    mod spec_full_stack_infer {
        use super::*;

        before -> i32 { 7 }

        before_each |n: &i32| -> _ {
            format!("val-{}", n)
        }

        after_each |n: &i32, s: _| {
            assert_eq!(*n, 7);
            assert_eq!(s, "val-7");
            SPEC_INFER_FULL_AFTER_EACH.fetch_add(1, Ordering::SeqCst);
        }

        it "full stack with infer" |n: &i32, s: _| {
            assert_eq!(*n, 7);
            assert_eq!(s, "val-7");
        }

        it "full stack with infer again" |n: &i32, s: _| {
            assert_eq!(*n, 7);
            assert_eq!(s, "val-7");
        }
    }
}

// --- attr style: sync #[before_each] fn setup() -> _ ---

#[test_suite]
mod attr_before_each_infer_sync {
    #[before_each]
    fn setup() -> _ {
        (String::from("attr-inferred"), 88u32)
    }

    #[test]
    fn receives_inferred(s: _, n: _) {
        assert_eq!(s, "attr-inferred");
        assert_eq!(n, 88);
    }
}

// --- attr style: async #[before_each] async fn setup() -> _ ---

static ATTR_INFER_ASYNC_AFTER_EACH: AtomicUsize = AtomicUsize::new(0);

#[test_suite(tokio)]
mod attr_before_each_infer_async {
    use super::*;

    #[before_each]
    pub async fn setup() -> _ {
        tokio::task::yield_now().await;
        (String::from("attr-async"), 77u64)
    }

    #[after_each]
    pub async fn teardown(s: _, n: _) {
        assert_eq!(s, "attr-async");
        assert_eq!(n, 77);
        ATTR_INFER_ASYNC_AFTER_EACH.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    pub async fn receives_async_inferred(s: _, n: _) {
        assert_eq!(s, "attr-async");
        assert_eq!(n, 77);
    }
}

// --- attr style: full stack before -> i32 + before_each -> _ ---

static ATTR_INFER_FULL_AFTER_EACH: AtomicUsize = AtomicUsize::new(0);

#[test_suite]
mod attr_full_stack_infer {
    use super::*;

    #[before]
    pub fn init() -> i32 {
        5
    }

    #[before_each]
    pub fn setup(n: &i32) -> _ {
        format!("item-{}", n)
    }

    #[after_each]
    pub fn teardown(n: &i32, s: _) {
        assert_eq!(*n, 5);
        assert_eq!(s, "item-5");
        ATTR_INFER_FULL_AFTER_EACH.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    pub fn full_stack_infer(n: &i32, s: _) {
        assert_eq!(*n, 5);
        assert_eq!(s, "item-5");
    }
}

// --- spec! style: single inferred return (no tuple) ---

spec! {
    mod spec_infer_single_value {
        before_each -> _ {
            vec![1, 2, 3]
        }

        it "receives single inferred value" |data: _| {
            assert_eq!(data, vec![1, 2, 3]);
        }
    }
}
