//! CLI for exercising the bat crate: loads a small (untrained, randomly
//! initialized) transformer, scales it to a target parameter count, and
//! runs real greedy autoregressive decoding over it.

use bat::{BatConfig, BatEngine};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let config = BatConfig { depth: 4, width: 64, vocab_size: 256, ..BatConfig::default() };
    let mut engine = BatEngine::load(config, &PathBuf::from("."))?;

    let prompt: Vec<u32> = vec![5, 12, 47];
    let generated = engine.generate(&prompt, 8);
    println!("Prompt: {:?}", prompt);
    println!("Generated (prompt + 8 new tokens): {:?}", generated);

    // Note: TransformerBlock::new eagerly materializes full random weight
    // matrices per block, so scaling to a multi-billion-parameter bucket
    // (e.g. the 7B+ buckets, which jump to width=1024+/depth=48+) would
    // allocate and randomly initialize hundreds of millions to billions of
    // f32s here -- far too slow/memory-heavy for a CLI demo. Use a modest
    // target to illustrate scaling without that blowup.
    engine.scale_to(600_000_000)?;
    println!("Scaled engine to the ~600M-parameter architecture bucket");

    let generated_after_scale = engine.generate(&prompt, 4);
    println!("Generated after scaling: {:?}", generated_after_scale);

    Ok(())
}
