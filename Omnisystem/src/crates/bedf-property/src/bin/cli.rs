//! CLI that runs the built-in properties against random inputs.

use bedf_property::{CommutativeProperty, IdempotentProperty, PropertyTestConfig, PropertyTester};

fn main() {
    let config = PropertyTestConfig {
        num_tests: 200,
        ..PropertyTestConfig::default()
    };
    let tester = PropertyTester::new(config);

    for result in [
        tester.run(&CommutativeProperty),
        tester.run(&IdempotentProperty),
    ] {
        println!(
            "{}: {} tests run, {} failure(s)",
            result.property_name,
            result.tests_run,
            result.failures.len()
        );
        if let Some(counterexample) = &result.shrunk_counterexample {
            println!("  shrunk counterexample: {counterexample:?}");
        }
    }
}
