use bedf_enhancements::{EnhancementEngine, EnhancementsConfig};

#[test]
fn test_full_catalog_round_trip() {
    let engine = EnhancementEngine::new(EnhancementsConfig::default());

    let all = engine.list_enhancements();
    assert_eq!(all.len(), 10);

    // Every catalog entry should have a non-empty name/description and be
    // independently addressable by id.
    for enhancement in &all {
        assert!(!enhancement.name.is_empty());
        assert!(!enhancement.description.is_empty());
        assert_eq!(engine.get_by_id(enhancement.id).unwrap().name, enhancement.name);
    }

    assert_eq!(engine.get_enabled().len(), 10);
}

#[test]
fn test_selectively_disabling_modules_via_config() {
    let config = EnhancementsConfig {
        enable_supply_chain: false,
        enable_hardened_sandbox: false,
        ..EnhancementsConfig::default()
    };

    let engine = EnhancementEngine::new(config);
    let enabled_names: Vec<String> = engine.get_enabled().into_iter().map(|e| e.name).collect();

    assert!(!enabled_names.contains(&"Supply Chain Attack Detection".to_string()));
    assert!(!enabled_names.contains(&"Hardened Sandboxes".to_string()));
    assert_eq!(enabled_names.len(), 8);
}
