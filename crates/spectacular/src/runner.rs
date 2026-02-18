use crate::report;
use crate::types::*;
use rand::SeedableRng;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::time::Instant;

pub fn run() -> SuiteResult {
    let start = Instant::now();

    let seed = match std::env::var("TEST_SEED") {
        Ok(s) => s.parse::<u64>().expect("TEST_SEED must be a valid u64"),
        Err(_) => rand::random(),
    };

    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    let suite_hooks: Vec<&SuiteHook> = inventory::iter::<SuiteHook>.into_iter().collect();
    let groups: Vec<&TestGroup> = inventory::iter::<TestGroup>.into_iter().collect();
    let tests: Vec<&TestCase> = inventory::iter::<TestCase>.into_iter().collect();

    let mut group_map: HashMap<&'static str, (Option<&TestGroup>, Vec<&TestCase>)> = HashMap::new();
    for group in &groups {
        group_map
            .entry(group.name)
            .or_insert_with(|| (None, Vec::new()))
            .0 = Some(group);
    }
    for test in &tests {
        group_map
            .entry(test.module)
            .or_insert_with(|| (None, Vec::new()))
            .1
            .push(test);
    }

    let mut group_names: Vec<&'static str> = group_map.keys().copied().collect();
    group_names.sort();
    group_names.shuffle(&mut rng);

    for (_name, (_group, tests)) in group_map.iter_mut() {
        tests.sort_by_key(|t| t.name);
        tests.shuffle(&mut rng);
    }

    let suite_befores: Vec<fn()> = suite_hooks
        .iter()
        .filter(|h| h.kind == SuiteHookKind::Before)
        .map(|h| h.hook_fn)
        .collect();
    let suite_afters: Vec<fn()> = suite_hooks
        .iter()
        .filter(|h| h.kind == SuiteHookKind::After)
        .map(|h| h.hook_fn)
        .collect();
    let suite_before_eachs: Vec<fn()> = suite_hooks
        .iter()
        .filter(|h| h.kind == SuiteHookKind::BeforeEach)
        .map(|h| h.hook_fn)
        .collect();
    let suite_after_eachs: Vec<fn()> = suite_hooks
        .iter()
        .filter(|h| h.kind == SuiteHookKind::AfterEach)
        .map(|h| h.hook_fn)
        .collect();

    report::print_header(seed);

    for hook in &suite_befores {
        hook();
    }

    let mut results = Vec::new();

    for group_name in &group_names {
        let (group, tests) = &group_map[group_name];

        report::print_group(group_name);

        if let Some(g) = group
            && let Some(before) = g.before
        {
            before();
        }

        for test in tests {
            let test_start = Instant::now();

            for hook in &suite_before_eachs {
                hook();
            }

            if let Some(g) = group
                && let Some(before_each) = g.before_each
            {
                before_each();
            }

            let outcome = match panic::catch_unwind(AssertUnwindSafe(test.test_fn)) {
                Ok(()) => TestOutcome::Passed,
                Err(e) => {
                    let msg = if let Some(s) = e.downcast_ref::<&str>() {
                        format!("{}\n  at {}:{}", s, test.file, test.line)
                    } else if let Some(s) = e.downcast_ref::<String>() {
                        format!("{}\n  at {}:{}", s, test.file, test.line)
                    } else {
                        format!("test panicked\n  at {}:{}", test.file, test.line)
                    };
                    TestOutcome::Failed(msg)
                }
            };

            if let Some(g) = group
                && let Some(after_each) = g.after_each
            {
                after_each();
            }

            for hook in &suite_after_eachs {
                hook();
            }

            let duration = test_start.elapsed();
            report::print_test_result(test.name, &outcome, duration);

            results.push(TestResult {
                name: test.name,
                module: test.module,
                file: test.file,
                line: test.line,
                outcome,
                duration,
            });
        }

        if let Some(g) = group
            && let Some(after) = g.after
        {
            after();
        }

        println!();
    }

    for hook in suite_afters.iter().rev() {
        hook();
    }

    let total_duration = start.elapsed();
    let suite_result = SuiteResult {
        results,
        seed,
        total_duration,
    };

    report::print_summary(&suite_result);

    suite_result
}
