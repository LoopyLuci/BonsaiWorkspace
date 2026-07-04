use crate::VerifiedProperty;
use core_ir::LairFunction;

pub fn generate_hints(func: &LairFunction, properties: &[VerifiedProperty]) -> Vec<OptimizationHint> {
    let mut hints = Vec::new();
    for prop in properties {
        match prop.property_type {
            crate::PropertyType::NoPanic => {
                hints.push(OptimizationHint {
                    hint_type: "remove_panic_check".into(),
                    confidence: 1.0,
                    description: format!("Verified: {}", prop.description),
                });
            }
            _ => {}
        }
    }
    hints
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OptimizationHint {
    pub hint_type: String,
    pub confidence: f32,
    pub description: String,
}
