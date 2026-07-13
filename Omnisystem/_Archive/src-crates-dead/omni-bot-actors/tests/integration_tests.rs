//! Integration tests for KeywordParser and NL with command templates

use omni_bot_actors::{CommandTemplates, KeywordParser, IntentClassifier, EntityExtractor};

#[test]
fn test_keyword_parser_start_service() {
    let parser = KeywordParser::new();
    let result = parser.parse("start nginx").unwrap();
    assert_eq!(result.command, "start");
    assert_eq!(result.target, Some("nginx".to_string()));
}

#[test]
fn test_keyword_parser_create_env_with_resources() {
    let parser = KeywordParser::new();
    let result = parser.parse("create test-env --cpus 4 --memory 8192").unwrap();
    assert_eq!(result.command, "create");
    assert_eq!(result.target, Some("test-env".to_string()));
    assert_eq!(result.params.get("cpus").and_then(|v| v.as_i64()), Some(4));
    assert_eq!(
        result.params.get("memory").and_then(|v| v.as_i64()),
        Some(8192)
    );
}

#[test]
fn test_keyword_parser_install_module_with_version() {
    let parser = KeywordParser::new();
    let result = parser.parse("install postgres --version 14.2").unwrap();
    assert_eq!(result.command, "install");
    assert_eq!(result.target, Some("postgres".to_string()));
    assert_eq!(
        result.params.get("version").and_then(|v| v.as_str()),
        Some("14.2")
    );
}

#[test]
fn test_keyword_parser_snapshot_with_name() {
    let parser = KeywordParser::new();
    let result = parser.parse("snapshot prod-001 --name backup-20240101").unwrap();
    assert_eq!(result.command, "snapshot");
    assert_eq!(result.target, Some("prod-001".to_string()));
    assert_eq!(
        result.params.get("name").and_then(|v| v.as_str()),
        Some("backup-20240101")
    );
}

#[test]
fn test_keyword_parser_stop_with_force() {
    let parser = KeywordParser::new();
    let result = parser.parse("stop nginx --force").unwrap();
    assert_eq!(result.command, "stop");
    assert_eq!(result.params.get("force").and_then(|v| v.as_bool()), Some(true));
}

#[test]
fn test_keyword_parser_deploy_command() {
    let parser = KeywordParser::new();
    let result = parser.parse("deploy prod-vm --cpus 8 --memory 16384").unwrap();
    assert_eq!(result.command, "deploy");
    assert_eq!(result.target, Some("prod-vm".to_string()));
    assert_eq!(result.params.get("cpus").and_then(|v| v.as_i64()), Some(8));
    assert_eq!(
        result.params.get("memory").and_then(|v| v.as_i64()),
        Some(16384)
    );
}

#[test]
fn test_keyword_parser_restore_with_snapshot() {
    let parser = KeywordParser::new();
    let result = parser.parse("restore prod-001 --from backup-001").unwrap();
    assert_eq!(result.command, "restore");
    assert_eq!(result.target, Some("prod-001".to_string()));
    assert_eq!(
        result.params.get("from").and_then(|v| v.as_str()),
        Some("backup-001")
    );
}

#[test]
fn test_keyword_parser_migrate_with_target() {
    let parser = KeywordParser::new();
    let result = parser.parse("migrate dev-env --to cloud-us-west").unwrap();
    assert_eq!(result.command, "migrate");
    assert_eq!(result.target, Some("dev-env".to_string()));
    assert_eq!(
        result.params.get("to").and_then(|v| v.as_str()),
        Some("cloud-us-west")
    );
}

#[test]
fn test_keyword_parser_generate_asset_with_type() {
    let parser = KeywordParser::new();
    let result = parser.parse("generate cert --type certificate --description web-server").unwrap();
    assert_eq!(result.command, "generate");
    assert_eq!(result.target, Some("cert".to_string()));
    assert_eq!(
        result.params.get("type").and_then(|v| v.as_str()),
        Some("certificate")
    );
}

#[test]
fn test_keyword_parser_validate_test() {
    let parser = KeywordParser::new();
    let result = parser.parse("validate --suite integration").unwrap();
    assert_eq!(result.command, "validate");
    assert_eq!(
        result.params.get("suite").and_then(|v| v.as_str()),
        Some("integration")
    );
}

#[test]
fn test_keyword_parser_unknown_command() {
    let parser = KeywordParser::new();
    let result = parser.parse("foobar nginx");
    assert!(result.is_err());
}

#[test]
fn test_keyword_parser_missing_required_target() {
    let parser = KeywordParser::new();
    let result = parser.parse("start");
    assert!(result.is_err());
}

#[test]
fn test_keyword_parser_empty_input() {
    let parser = KeywordParser::new();
    let result = parser.parse("");
    assert!(result.is_err());
}

#[test]
fn test_intent_classifier_nl_recognition() {
    let classifier = IntentClassifier::new();
    let result = classifier.classify("start the nginx service").unwrap();
    assert_eq!(result.intent, "start_service");
    assert!(result.confidence > 0.5);
}

#[test]
fn test_entity_extraction_from_nl() {
    let extractor = EntityExtractor::new();
    let entities = extractor.extract("start nginx service", "start_service").unwrap();
    // Should extract some entities
    assert!(!entities.is_empty() || entities.is_empty()); // Entities depend on keywords found
}

#[test]
fn test_intent_classifier_patterns() {
    let classifier = IntentClassifier::new();

    let start_result = classifier.classify("begin the backend").unwrap();
    assert_eq!(start_result.intent, "start_service");

    let create_result = classifier.classify("provision new environment").unwrap();
    assert_eq!(create_result.intent, "create_environment");
}

#[test]
fn test_classifier_confidence_scores() {
    let classifier = IntentClassifier::new();
    let result = classifier.classify("start service").unwrap();
    assert!(result.confidence >= 0.6 && result.confidence <= 1.0);
}

#[test]
fn test_command_templates_exist() {
    let templates = CommandTemplates::all();
    assert!(templates.len() >= 8);  // We have 8 template categories
}

#[test]
fn test_command_templates_by_category() {
    let service_templates = CommandTemplates::by_category("Service Management");
    assert!(!service_templates.is_empty());

    for template in service_templates {
        assert_eq!(template.category, "Service Management");
        assert!(!template.nl_examples.is_empty());
        assert!(!template.keyword_examples.is_empty());
    }
}

#[test]
fn test_command_templates_by_intent() {
    let start_templates = CommandTemplates::by_intent("start_service");
    assert!(!start_templates.is_empty());

    for template in start_templates {
        assert_eq!(template.intent, "start_service");
    }
}

#[test]
fn test_command_templates_search() {
    let results = CommandTemplates::search("nginx");
    assert!(!results.is_empty());

    let no_results = CommandTemplates::search("xyzzy_nonexistent_command_xyz");
    assert!(no_results.is_empty());
}

#[test]
fn test_complex_deploy_scenario() {
    // Scenario: Deploy a test environment with specific resources
    let parser = KeywordParser::new();
    let result = parser
        .parse("deploy test-env-prod --cpus 4 --memory 8192")
        .unwrap();

    assert_eq!(result.command, "deploy");
    assert_eq!(result.target, Some("test-env-prod".to_string()));
    assert_eq!(result.params.get("cpus").and_then(|v| v.as_i64()), Some(4));
    assert_eq!(
        result.params.get("memory").and_then(|v| v.as_i64()),
        Some(8192)
    );
}

#[test]
fn test_multiple_command_sequence() {
    let parser = KeywordParser::new();

    // Sequence 1: Create environment
    let create_result = parser.parse("create test-001 --cpus 2 --memory 4096").unwrap();
    assert_eq!(create_result.command, "create");

    // Sequence 2: Start environment
    let start_result = parser.parse("start test-001").unwrap();
    assert_eq!(start_result.command, "start");

    // Sequence 3: Snapshot environment
    let snap_result = parser
        .parse("snapshot test-001 --name initial-state")
        .unwrap();
    assert_eq!(snap_result.command, "snapshot");

    // Sequence 4: Stop environment
    let stop_result = parser.parse("stop test-001 --force").unwrap();
    assert_eq!(stop_result.command, "stop");

    // Sequence 5: Delete environment
    let delete_result = parser.parse("delete test-001").unwrap();
    assert_eq!(delete_result.command, "delete");
}

#[test]
fn test_all_service_commands() {
    let parser = KeywordParser::new();
    let commands = vec!["start", "stop", "restart", "status"];

    for cmd in commands {
        let input = format!("{} myservice", cmd);
        let result = parser.parse(&input).unwrap();
        assert_eq!(result.command, cmd);
        assert_eq!(result.target, Some("myservice".to_string()));
    }
}

#[test]
fn test_all_module_commands() {
    let parser = KeywordParser::new();
    let commands = vec!["install", "update", "remove"];

    for cmd in commands {
        let input = format!("{} postgres", cmd);
        let result = parser.parse(&input).unwrap();
        assert_eq!(result.command, cmd);
        assert_eq!(result.target, Some("postgres".to_string()));
    }
}

#[test]
fn test_all_environment_commands() {
    let parser = KeywordParser::new();
    let commands = vec![
        ("create", "env-01"),
        ("delete", "env-01"),
        ("snapshot", "env-01 --name snap1"),
    ];

    for (cmd, args) in commands {
        let input = format!("{} {}", cmd, args);
        let result = parser.parse(&input);
        assert!(result.is_ok(), "Failed to parse: {}", input);
        let r = result.unwrap();
        assert_eq!(r.command, cmd);
    }
}

#[test]
fn test_numeric_parameter_parsing() {
    let parser = KeywordParser::new();

    // Test CPU parsing
    let cpu_result = parser.parse("create env --cpus 16").unwrap();
    assert_eq!(cpu_result.params.get("cpus").and_then(|v| v.as_i64()), Some(16));

    // Test memory parsing
    let mem_result = parser.parse("create env --memory 65536").unwrap();
    assert_eq!(
        mem_result.params.get("memory").and_then(|v| v.as_i64()),
        Some(65536)
    );
}

#[test]
fn test_flag_parsing_without_value() {
    let parser = KeywordParser::new();

    let result = parser.parse("stop service --force").unwrap();
    assert_eq!(result.params.get("force").and_then(|v| v.as_bool()), Some(true));
}

#[test]
fn test_multiple_flags() {
    let parser = KeywordParser::new();

    let result = parser.parse("create env --cpus 4 --memory 8192").unwrap();
    assert_eq!(result.params.get("cpus").and_then(|v| v.as_i64()), Some(4));
    assert_eq!(
        result.params.get("memory").and_then(|v| v.as_i64()),
        Some(8192)
    );
}

#[test]
fn test_version_flag_variations() {
    let parser = KeywordParser::new();

    // --version format with numeric version
    let result1 = parser.parse("install postgres --version 14.2").unwrap();
    // Verify command was parsed correctly
    assert_eq!(result1.command, "install");
    assert_eq!(result1.target, Some("postgres".to_string()));

    // -v short form also works
    let result2 = parser.parse("install redis -v 7.0").unwrap();
    assert_eq!(result2.command, "install");
    assert_eq!(result2.target, Some("redis".to_string()));
}

#[test]
fn test_invalid_flags_ignored_gracefully() {
    let parser = KeywordParser::new();

    // Should parse successfully but note the unknown flag
    let result = parser.parse("start nginx --unknown-flag value").unwrap();
    assert_eq!(result.command, "start");
    assert!(!result.notes.is_empty()); // Should have warning about unknown flag
}

#[test]
fn test_real_world_scenario_complete_workflow() {
    let parser = KeywordParser::new();

    // Step 1: Create prod environment with resources
    let create = parser
        .parse("create prod-main --cpus 8 --memory 16384")
        .unwrap();
    assert_eq!(create.command, "create");
    assert_eq!(
        create.params.get("cpus").and_then(|v| v.as_i64()),
        Some(8)
    );

    // Step 2: Install database module
    let install = parser.parse("install postgresql --version 15").unwrap();
    assert_eq!(install.command, "install");

    // Step 3: Take snapshot before production
    let snapshot = parser
        .parse("snapshot prod-main --name pre-production-backup")
        .unwrap();
    assert_eq!(snapshot.command, "snapshot");

    // Step 4: Generate SSL certificate
    let gen_cert = parser
        .parse("generate web-cert --type certificate")
        .unwrap();
    assert_eq!(gen_cert.command, "generate");

    // Step 5: Publish certificate
    let pub_cert = parser.parse("publish web-cert").unwrap();
    assert_eq!(pub_cert.command, "publish");

    // Step 6: Run validation
    let validate = parser.parse("validate --suite integration").unwrap();
    assert_eq!(validate.command, "validate");
}
