//! CLI that exercises real JSON RPC message encode/decode.

use omnisystem_network::{ProtocolHandler, RPCMessage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let handler = ProtocolHandler::new()?;

    let request = RPCMessage::request("ping", serde_json::json!({"nonce": 42}));
    let encoded = handler.encode(&request)?;
    println!("Encoded {} bytes", encoded.len());

    let decoded = handler.decode(&encoded)?;
    println!("Decoded method: {}", decoded.method);
    println!("Decoded params: {}", decoded.params);

    Ok(())
}
