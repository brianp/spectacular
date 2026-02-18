pub struct TestCase {
    pub name: &'static str,
    pub module: &'static str,
    pub test_fn: fn(),
    pub file: &'static str,
    pub line: u32,
}

pub struct TestGroup {
    pub name: &'static str,
    pub before: Option<fn()>,
    pub after: Option<fn()>,
    pub before_each: Option<fn()>,
    pub after_each: Option<fn()>,
}

pub struct SuiteHook {
    pub kind: SuiteHookKind,
    pub hook_fn: fn(),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuiteHookKind {
    Before,
    After,
    BeforeEach,
    AfterEach,
}

#[derive(Debug)]
pub struct TestResult {
    pub name: &'static str,
    pub module: &'static str,
    pub file: &'static str,
    pub line: u32,
    pub outcome: TestOutcome,
    pub duration: std::time::Duration,
}

#[derive(Debug)]
pub enum TestOutcome {
    Passed,
    Failed(String),
}

#[derive(Debug)]
pub struct SuiteResult {
    pub results: Vec<TestResult>,
    pub seed: u64,
    pub total_duration: std::time::Duration,
}

impl SuiteResult {
    pub fn passed(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.outcome, TestOutcome::Passed))
            .count()
    }

    pub fn failed(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.outcome, TestOutcome::Failed(_)))
            .count()
    }

    pub fn all_passed(&self) -> bool {
        self.results
            .iter()
            .all(|r| matches!(r.outcome, TestOutcome::Passed))
    }
}

inventory::collect!(TestCase);
inventory::collect!(TestGroup);
inventory::collect!(SuiteHook);
