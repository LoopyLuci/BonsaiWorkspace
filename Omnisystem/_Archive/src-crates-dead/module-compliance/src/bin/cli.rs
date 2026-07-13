use module_compliance::{ComplianceFramework, ComplianceManager, FrameworkConfig};

fn main() {
    println!("Module Compliance Manager - v{}", env!("CARGO_PKG_VERSION"));
    println!("Enterprise Compliance Automation");
    println!();
    println!("Supported Frameworks:");
    println!("  ✓ HIPAA (Healthcare)");
    println!("  ✓ SOC2 (Security & Availability)");
    println!("  ✓ GDPR (Data Protection)");
    println!("  ✓ CCPA (Privacy)");
    println!("  ✓ PCI-DSS (Payment Card)");
    println!("  ✓ ISO27001 (Information Security)");
    println!("  ✓ FedRAMP (Government)");
    println!();
    println!("Creating compliance manager...");
    let manager = ComplianceManager::new();

    let hipaa_config = FrameworkConfig {
        framework: ComplianceFramework::HIPAA,
        enabled: true,
        review_interval_days: 90,
        auto_remediate: true,
        notification_email: Some("compliance@omnisystem.io".to_string()),
    };

    manager.enable_framework(ComplianceFramework::HIPAA, hipaa_config).unwrap();
    println!("Compliance frameworks enabled");
    println!();
    println!("Status: Enterprise compliance controls active");
}
