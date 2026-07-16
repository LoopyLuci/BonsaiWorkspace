//! Buddy CLI: creates an assistant, registers a custom capability, sets
//! some conversation context, and runs an interaction.

use buddy::Buddy;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let buddy = Buddy::new("Buddy".to_string());
    buddy.register_capability("weather".to_string(), "Report the local weather".to_string())?;
    buddy.set_context("user_name".to_string(), "Alice".to_string());

    println!(
        "{} has {} capabilities registered",
        buddy.get_name(),
        buddy.list_capabilities().len()
    );

    let response = buddy.interact("What's the weather like?".to_string()).await?;
    println!("Response: {}", response);
    println!("Conversation length: {}", buddy.conversation_length());

    Ok(())
}
