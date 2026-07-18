//! CLI for exercising the stream-processor crate.

use stream_processor::{StreamProcessor, WindowType};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let processor = StreamProcessor::new();

    let mut data = HashMap::new();
    data.insert("price".to_string(), "104.20".to_string());
    processor.emit_event("trades", data).await?;

    let mut data2 = HashMap::new();
    data2.insert("price".to_string(), "104.55".to_string());
    processor.emit_event("trades", data2).await?;

    let window = processor.create_window("trades", WindowType::Tumbling, 60_000).await?;
    let agg = processor.aggregate(window.window_id, "avg", &[104.20, 104.55]).await?;
    println!("Average price in window: {:.2}", agg.result);

    let processed = processor.process_stream("trades", "count").await?;
    println!("Processed '{}' -> {} (latency {}ms)", processed.operation, processed.output, processed.latency_ms);

    println!("Total events buffered: {}", processor.event_count());
    Ok(())
}
