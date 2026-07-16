use bedf_property::{CommutativeProperty, IdempotentProperty, PropertyTestConfig, PropertyTester};

#[test]
fn test_built_in_properties_hold_over_many_random_inputs() {
    let config = PropertyTestConfig {
        num_tests: 100,
        ..PropertyTestConfig::default()
    };
    let tester = PropertyTester::new(config);

    let commutative_result = tester.run(&CommutativeProperty);
    assert_eq!(commutative_result.tests_run, 100);
    assert!(commutative_result.failures.is_empty());

    let idempotent_result = tester.run(&IdempotentProperty);
    assert_eq!(idempotent_result.tests_run, 100);
    assert!(idempotent_result.failures.is_empty());
}
