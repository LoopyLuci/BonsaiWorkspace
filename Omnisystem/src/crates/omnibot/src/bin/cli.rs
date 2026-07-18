//! OmniBot CLI - exercises the command registry, intent classifier, and permissions

use omnibot::{
    Capability, CommandRegistry, HelpCommand, IntentClassifier, Platform, PingCommand, User,
    UserId, UserRole,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let registry = CommandRegistry::new();
    registry.register(Arc::new(HelpCommand));
    registry.register(Arc::new(PingCommand));
    println!("registered commands: {}", registry.list().len());

    if let Some(help) = registry.get("help") {
        println!("found command: {} - {}", help.name(), help.description());
    }

    let classifier = IntentClassifier::new();
    let intent = classifier.classify("show me status").await?;
    println!("classified intent: {}", intent.description());

    let user = User::new(UserId::telegram("123"), UserRole::Operator, Platform::Telegram);
    println!(
        "user has Deploy capability: {}",
        user.has_capability(&Capability::Deploy)
    );

    Ok(())
}
