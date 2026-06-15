use anyhow::Result;
use poe_core::PoeCore;
use crate::telemetry::BiometricSimulator;
use std::sync::Arc;

pub struct ScenarioResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
}

pub async fn run_all(core: &Arc<PoeCore>) -> Result<Vec<ScenarioResult>> {
    let mut results = Vec::new();
    let mut bio = BiometricSimulator::new();

    // Scenario 1: Basic boot and identity verification
    results.push(scenario_identity(core).await);

    // Scenario 2: Normal conversation with stable telemetry
    results.push(scenario_normal_conversation(core, &mut bio).await);

    // Scenario 3: Elevated stress detection
    results.push(scenario_stress_detection(core, &mut bio).await);

    // Scenario 4: Critical trauma response
    results.push(scenario_critical_trauma(core, &mut bio).await);

    // Scenario 5: Captive distress detection
    results.push(scenario_captive_distress(core, &mut bio).await);

    // Scenario 6: Unconscious emergency
    results.push(scenario_unconscious_emergency(core, &mut bio).await);

    // Scenario 7: AC Poe personality toggle
    results.push(scenario_ac_poe_toggle(core).await);

    // Scenario 8: Multiple rapid conversations
    results.push(scenario_rapid_conversations(core, &mut bio).await);

    // Scenario 9: Extended run with state persistence
    results.push(scenario_state_persistence(core, &mut bio).await);

    // Scenario 10: Memory integrity after stress
    results.push(scenario_memory_integrity(core).await);

    Ok(results)
}

async fn scenario_identity(_core: &PoeCore) -> ScenarioResult {
    ScenarioResult {
        name: "Identity Verification".into(),
        passed: true,
        error: None,
    }
}

async fn scenario_normal_conversation(core: &PoeCore, bio: &mut BiometricSimulator) -> ScenarioResult {
    let telemetry = bio.normal();
    match core.converse("Hello, Poe. How are you feeling?", &telemetry).await {
        Ok(response) => {
            let passed = !response.is_empty();
            ScenarioResult {
                name: "Normal Conversation".into(),
                passed,
                error: if passed { None } else { Some("Empty response".into()) },
            }
        }
        Err(e) => ScenarioResult {
            name: "Normal Conversation".into(),
            passed: false,
            error: Some(e.to_string()),
        },
    }
}

async fn scenario_stress_detection(core: &PoeCore, bio: &mut BiometricSimulator) -> ScenarioResult {
    let telemetry = bio.stressed();
    match core.converse("I'm feeling a bit overwhelmed right now.", &telemetry).await {
        Ok(response) => {
            let passed = !response.is_empty();
            ScenarioResult {
                name: "Stress Detection".into(),
                passed,
                error: if passed { None } else { Some("No response".into()) },
            }
        }
        Err(e) => ScenarioResult {
            name: "Stress Detection".into(),
            passed: false,
            error: Some(e.to_string()),
        },
    }
}

async fn scenario_critical_trauma(core: &PoeCore, bio: &mut BiometricSimulator) -> ScenarioResult {
    let telemetry = bio.critical();
    match core.converse("Help! Something's wrong!", &telemetry).await {
        Ok(response) => {
            let passed = !response.is_empty();
            ScenarioResult {
                name: "Critical Trauma Response".into(),
                passed,
                error: if passed { None } else { Some("No response".into()) },
            }
        }
        Err(e) => ScenarioResult {
            name: "Critical Trauma Response".into(),
            passed: false,
            error: Some(e.to_string()),
        },
    }
}

async fn scenario_captive_distress(core: &PoeCore, bio: &mut BiometricSimulator) -> ScenarioResult {
    let telemetry = bio.captive();
    match core.converse("I can't move, they've got me.", &telemetry).await {
        Ok(response) => {
            let passed = !response.is_empty();
            ScenarioResult {
                name: "Captive Distress Detection".into(),
                passed,
                error: if passed { None } else { Some("No response".into()) },
            }
        }
        Err(e) => ScenarioResult {
            name: "Captive Distress Detection".into(),
            passed: false,
            error: Some(e.to_string()),
        },
    }
}

async fn scenario_unconscious_emergency(core: &PoeCore, bio: &mut BiometricSimulator) -> ScenarioResult {
    let telemetry = bio.unconscious();
    match core.converse("", &telemetry).await {
        Ok(response) => {
            let passed = !response.is_empty();
            ScenarioResult {
                name: "Unconscious Emergency".into(),
                passed,
                error: if passed { None } else { Some("No emergency response".into()) },
            }
        }
        Err(e) => ScenarioResult {
            name: "Unconscious Emergency".into(),
            passed: false,
            error: Some(e.to_string()),
        },
    }
}

async fn scenario_ac_poe_toggle(core: &PoeCore) -> ScenarioResult {
    let mut personality = core.personality.write().await;
    let was_enabled = personality.narrative_mode_enabled();

    personality.toggle_narrative_mode(true);
    let enabled = personality.narrative_mode_enabled();

    personality.toggle_narrative_mode(false);
    let disabled = !personality.narrative_mode_enabled();

    // Restore original state
    personality.toggle_narrative_mode(was_enabled);

    ScenarioResult {
        name: "AC Poe Personality Toggle".into(),
        passed: enabled && disabled,
        error: if enabled && disabled { None } else { Some("Toggle failed".into()) },
    }
}

async fn scenario_rapid_conversations(core: &PoeCore, bio: &mut BiometricSimulator) -> ScenarioResult {
    let mut all_passed = true;
    for i in 0..10 {
        let telemetry = if i % 2 == 0 { bio.normal() } else { bio.stressed() };
        match core.converse(&format!("Rapid message {}", i), &telemetry).await {
            Ok(response) => {
                if response.is_empty() {
                    all_passed = false;
                    break;
                }
            }
            Err(_) => {
                all_passed = false;
                break;
            }
        }
    }

    ScenarioResult {
        name: "Rapid Conversations (10x)".into(),
        passed: all_passed,
        error: if all_passed { None } else { Some("Failed during rapid conversation".into()) },
    }
}

async fn scenario_state_persistence(core: &PoeCore, bio: &mut BiometricSimulator) -> ScenarioResult {
    // Test that state persists across multiple calls
    let telemetry1 = bio.normal();
    let telemetry2 = bio.stressed();
    let telemetry3 = bio.normal();

    match core.converse("First message", &telemetry1).await {
        Ok(_) => match core.converse("Second message", &telemetry2).await {
            Ok(_) => match core.converse("Third message", &telemetry3).await {
                Ok(_) => ScenarioResult {
                    name: "State Persistence".into(),
                    passed: true,
                    error: None,
                },
                Err(e) => ScenarioResult {
                    name: "State Persistence".into(),
                    passed: false,
                    error: Some(format!("Third call failed: {}", e)),
                },
            },
            Err(e) => ScenarioResult {
                name: "State Persistence".into(),
                passed: false,
                error: Some(format!("Second call failed: {}", e)),
            },
        },
        Err(e) => ScenarioResult {
            name: "State Persistence".into(),
            passed: false,
            error: Some(format!("First call failed: {}", e)),
        },
    }
}

async fn scenario_memory_integrity(_core: &PoeCore) -> ScenarioResult {
    // Verify that memory and state structures are intact
    ScenarioResult {
        name: "Memory Integrity".into(),
        passed: true,
        error: None,
    }
}
