//! CLI demo: record GDPR consent and scan text for sensitive data.

use chrono::Utc;
use data_privacy::{DataLossPreventionEngine, DlpRule, GdprManager, UserConsent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gdpr = GdprManager::new();
    let dlp = DataLossPreventionEngine::new();

    gdpr.record_consent(&UserConsent {
        user_id: "u1".to_string(),
        consent_type: "marketing".to_string(),
        given_at: Utc::now(),
        expires_at: None,
    })
    .await?;
    println!("Consent recorded: {}", gdpr.check_consent("u1").await?);

    dlp.add_rule(&DlpRule {
        rule_id: "r1".to_string(),
        pattern: "ssn".to_string(),
        action: "block".to_string(),
    })
    .await?;
    match dlp.scan_data("contains ssn: 123-45-6789").await {
        Ok(_) => println!("No sensitive data detected"),
        Err(e) => println!("Sensitive data detected: {}", e),
    }

    Ok(())
}
