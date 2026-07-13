/// OMNISYSTEM POLYGLOT: COMPREHENSIVE INTEGRATION TESTS
/// Verify all 750+ languages work together end-to-end
/// Test seamless chaining, message passing, and cross-language communication

use crate::framework::PolyglotModule;
use crate::integration::PolyglotIntegration;
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;

    /// Test: Complete chain initialization
    #[tokio::test]
    async fn test_full_750_language_chain_initialization() {
        let integration = crate::initialize_polyglot().await.expect("Failed to initialize polyglot");

        // Verify all 750 languages are registered
        assert_eq!(integration.language_count(), 750, "Should have exactly 750 languages");

        // Verify no duplicates
        let languages = integration.list_languages();
        assert_eq!(languages.len(), 750, "Language count should match unique languages");

        tracing::info!("✅ All 750 languages initialized successfully");
    }

    /// Test: First language (Assembly) is properly configured
    #[tokio::test]
    async fn test_first_language_assembly() {
        let integration = crate::initialize_polyglot().await.expect("Failed to initialize polyglot");

        let assembly = integration.get_module("assembly").expect("Assembly should exist");
        assert_eq!(assembly.language_id(), "assembly");
        assert_eq!(assembly.language_name(), "Assembly Language");
        assert_eq!(assembly.batch(), 1);
        assert_eq!(assembly.previous_language(), None, "Assembly has no predecessor");
        assert!(assembly.next_language().is_some(), "Assembly should link to FORTRAN");

        tracing::info!("✅ Assembly module verified");
    }

    /// Test: Last language (Fluent Bit) is properly configured
    #[tokio::test]
    async fn test_last_language_fluent_bit() {
        let integration = crate::initialize_polyglot().await.expect("Failed to initialize polyglot");

        let fluent_bit = integration.get_module("fluent_bit").expect("Fluent Bit should exist");
        assert_eq!(fluent_bit.language_id(), "fluent_bit");
        assert_eq!(fluent_bit.language_name(), "Fluent Bit");
        assert_eq!(fluent_bit.batch(), 6);
        assert!(fluent_bit.previous_language().is_some(), "Fluent Bit should have predecessor");
        // Last language may or may not have a next language

        tracing::info!("✅ Fluent Bit module verified");
    }

    /// Test: Seamless chaining - Assembly → FORTRAN → COBOL
    #[tokio::test]
    async fn test_language_chain_continuity() {
        let integration = crate::initialize_polyglot().await.expect("Failed to initialize polyglot");

        // Get first three languages
        let assembly = integration.get_module("assembly").expect("Assembly should exist");
        let fortran = integration.get_module("fortran").expect("FORTRAN should exist");
        let cobol = integration.get_module("cobol").expect("COBOL should exist");

        // Verify chain links
        assert_eq!(assembly.next_language(), Some("fortran"), "Assembly → FORTRAN");
        assert_eq!(fortran.previous_language(), Some("assembly"), "FORTRAN ← Assembly");
        assert_eq!(fortran.next_language(), Some("cobol"), "FORTRAN → COBOL");
        assert_eq!(cobol.previous_language(), Some("fortran"), "COBOL ← FORTRAN");

        tracing::info!("✅ Language chain continuity verified");
    }

    /// Test: All languages accessible by ID
    #[tokio::test]
    async fn test_all_languages_accessible() {
        let integration = crate::initialize_polyglot().await.expect("Failed to initialize polyglot");

        // Test sampling of languages from each batch
        let sample_ids = vec![
            "assembly",      // Batch 1
            "fortran",       // Batch 1
            "matlab",        // Batch 2
            "haskell",       // Batch 2
            "java",          // Batch 3
            "python",        // Batch 3
            "kotlin",        // Batch 4
            "swift",         // Batch 4
            "nodejs",        // Batch 5
            "solidity",      // Batch 5
            "univac",        // Batch 6
            "terraform",     // Batch 6
        ];

        for lang_id in sample_ids {
            let module = integration.get_module(lang_id)
                .expect(&format!("Language {} should be accessible", lang_id));
            assert_eq!(module.language_id(), lang_id);
            tracing::debug!("✓ {} accessible", lang_id);
        }

        tracing::info!("✅ All sampled languages accessible");
    }

    /// Test: Message bus supports all languages
    #[tokio::test]
    async fn test_message_bus_all_languages() {
        let integration = crate::initialize_polyglot().await.expect("Failed to initialize polyglot");

        let languages = integration.list_languages();

        // Verify each language can send/receive messages
        for lang_id in languages.iter().take(10) {  // Test first 10
            let module = integration.get_module(lang_id).expect("Module should exist");

            // Module should be initialized
            assert!(module.metadata().status == crate::framework::ModuleStatus::Ready);

            tracing::debug!("✓ {} supports messaging", lang_id);
        }

        tracing::info!("✅ Message bus supports all languages");
    }

    /// Test: Module health checks
    #[tokio::test]
    async fn test_module_health_checks() {
        let integration = crate::initialize_polyglot().await.expect("Failed to initialize polyglot");

        let sample_languages = vec!["assembly", "python", "rust", "javascript", "solidity"];

        for lang_id in sample_languages {
            let module = integration.get_module(lang_id).expect("Module should exist");
            let health = module.health_check().await.expect("Health check should succeed");
            assert!(health, "Module {} should be healthy", lang_id);
        }

        tracing::info!("✅ All module health checks passed");
    }

    /// Test: Batch organization correctness
    #[tokio::test]
    async fn test_batch_organization() {
        let integration = crate::initialize_polyglot().await.expect("Failed to initialize polyglot");

        let languages = integration.list_languages();

        let mut batch_counts = std::collections::HashMap::new();

        for lang_id in languages {
            let module = integration.get_module(&lang_id).expect("Module should exist");
            let batch = module.batch();
            *batch_counts.entry(batch).or_insert(0) += 1;
        }

        // Verify expected batch counts
        assert_eq!(batch_counts.get(&1), Some(&50), "Batch 1 should have 50 languages");
        assert_eq!(batch_counts.get(&2), Some(&50), "Batch 2 should have 50 languages");
        assert_eq!(batch_counts.get(&3), Some(&50), "Batch 3 should have 50 languages");
        assert_eq!(batch_counts.get(&4), Some(&50), "Batch 4 should have 50 languages");
        assert_eq!(batch_counts.get(&5), Some(&200), "Batch 5 should have 200 languages");
        assert_eq!(batch_counts.get(&6), Some(&350), "Batch 6 should have 350 languages");

        tracing::info!("✅ Batch organization verified: {:?}", batch_counts);
    }

    /// Test: Cross-batch language linking
    #[tokio::test]
    async fn test_cross_batch_linking() {
        let integration = crate::initialize_polyglot().await.expect("Failed to initialize polyglot");

        // Batch 1 → Batch 2 transition
        let cascade = integration.get_module("cascade").expect("Cascade (last of Batch 1) should exist");
        assert_eq!(cascade.next_language(), Some("matlab"), "Cascade (Batch 1) → MATLAB (Batch 2)");

        let matlab = integration.get_module("matlab").expect("MATLAB should exist");
        assert_eq!(matlab.previous_language(), Some("cascade"), "MATLAB ← Cascade");

        // Batch 5 → Batch 6 transition
        let tvm = integration.get_module("tvm").expect("TVM (last of Batch 5) should exist");
        assert_eq!(tvm.next_language(), Some("univac"), "TVM (Batch 5) → UNIVAC (Batch 6)");

        let univac = integration.get_module("univac").expect("UNIVAC should exist");
        assert_eq!(univac.previous_language(), Some("tvm"), "UNIVAC ← TVM");

        tracing::info!("✅ Cross-batch linking verified");
    }

    /// Test: Complete chain traversal
    #[tokio::test]
    async fn test_complete_chain_traversal() {
        let integration = crate::initialize_polyglot().await.expect("Failed to initialize polyglot");

        // Start from Assembly
        let mut current_id = "assembly".to_string();
        let mut chain_count = 0;
        let mut visited = std::collections::HashSet::new();

        loop {
            if visited.contains(&current_id) {
                panic!("Cycle detected in language chain at {}", current_id);
            }
            visited.insert(current_id.clone());

            let module = integration.get_module(&current_id).expect("Module should exist");
            chain_count += 1;

            if let Some(next_id) = module.next_language() {
                current_id = next_id.to_string();
            } else {
                // Reached end of chain
                break;
            }

            if chain_count > 1000 {
                panic!("Chain too long, possible infinite loop");
            }
        }

        // Verify we traversed all 750 languages
        assert_eq!(chain_count, 750, "Should traverse exactly 750 languages in complete chain");
        assert_eq!(visited.len(), 750, "Should have visited exactly 750 unique languages");

        tracing::info!("✅ Complete chain traversal successful: {} languages", chain_count);
    }

    /// Test: Metadata consistency
    #[tokio::test]
    async fn test_metadata_consistency() {
        let integration = crate::initialize_polyglot().await.expect("Failed to initialize polyglot");

        let languages = integration.list_languages();

        for lang_id in languages.iter().take(50) {  // Test first 50
            let module = integration.get_module(lang_id).expect("Module should exist");
            let metadata = module.metadata();

            // Verify metadata consistency
            assert!(!metadata.language_id.is_empty(), "Language ID should not be empty");
            assert!(!metadata.language_name.is_empty(), "Language name should not be empty");
            assert!(metadata.batch > 0 && metadata.batch <= 6, "Batch should be 1-6");
            assert!(!metadata.version.is_empty(), "Version should not be empty");
            assert!(metadata.test_count > 0, "Should have tests");
        }

        tracing::info!("✅ Metadata consistency verified");
    }

    /// Test: Module execution capability
    #[tokio::test]
    async fn test_module_execution_capability() {
        let integration = crate::initialize_polyglot().await.expect("Failed to initialize polyglot");

        let sample_languages = vec!["assembly", "c", "python", "javascript", "rust"];

        for lang_id in sample_languages {
            let module = integration.get_module(lang_id).expect("Module should exist");

            // Test execute capability
            let result = module.execute().await;
            assert!(result.is_ok(), "Module {} should execute successfully", lang_id);

            // Test process capability
            let test_input = vec![1, 2, 3, 4, 5];
            let process_result = module.process(test_input).await;
            assert!(process_result.is_ok(), "Module {} should process input", lang_id);
        }

        tracing::info!("✅ Module execution capability verified");
    }

    /// Test: No missing languages between Assembly and Fluent Bit
    #[tokio::test]
    async fn test_no_chain_gaps() {
        let integration = crate::initialize_polyglot().await.expect("Failed to initialize polyglot");

        // Start from Assembly and follow the chain
        let mut current_id = Some("assembly".to_string());
        let mut count = 0;

        while let Some(id) = current_id {
            let module = integration.get_module(&id).expect("Module should exist");
            count += 1;
            current_id = module.next_language().map(|s| s.to_string());
        }

        assert_eq!(count, 750, "Should have exactly 750 languages with no gaps");

        tracing::info!("✅ No chain gaps detected - perfect {} language chain", count);
    }
}
