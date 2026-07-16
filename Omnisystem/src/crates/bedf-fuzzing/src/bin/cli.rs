//! bedf-fuzzing CLI: runs a real coverage-guided mutation fuzz loop against
//! a small toy target, using the corpus, mutator, and coverage tracker
//! together end to end.

use bedf_fuzzing::{Corpus, CoverageGuidedFuzzer, FuzzerConfig, MutationStrategy, Mutator};

/// A toy fuzz target: "crashes" (returns Err) on a specific 3-byte pattern,
/// so the fuzzer has something real to discover, and reports which
/// branches ("edges") it took so coverage can be tracked.
fn run_target(input: &[u8]) -> Result<Vec<u64>, &'static str> {
    let mut edges = vec![1]; // entry

    if input.is_empty() {
        edges.push(2);
        return Ok(edges);
    }
    edges.push(3);

    if input[0] == 0x7F {
        edges.push(4);
        if input.len() > 1 && input[1] == 0xFF {
            edges.push(5);
            if input.len() > 2 && input[2] == 0x00 {
                edges.push(6);
                return Err("crash: reserved sequence 7F FF 00 detected");
            }
        }
    } else {
        edges.push(7);
    }

    Ok(edges)
}

fn mutation_strategy_for(i: usize) -> MutationStrategy {
    match i % 5 {
        0 => MutationStrategy::BitFlip,
        1 => MutationStrategy::ByteFlip,
        2 => MutationStrategy::Interesting,
        3 => MutationStrategy::Dictionary,
        _ => MutationStrategy::Havoc,
    }
}

fn main() {
    let mut corpus = Corpus::new();
    corpus.add_corpus(b"seed");
    corpus.add_corpus(&[0x00, 0x01, 0x02]);
    // Seed something close to the planted crash so the mutator has a
    // realistic chance of finding it within a modest iteration budget --
    // real coverage-guided fuzzers rely on exactly this kind of "nudge"
    // (a near-miss seed) to make rare targets reachable.
    corpus.add_corpus(&[0x7F, 0xFF, 0x01]);

    let config = FuzzerConfig {
        max_coverage: 7,
        max_iterations: 5000,
        ..FuzzerConfig::default()
    };
    let mut fuzzer = CoverageGuidedFuzzer::new(config.clone());

    let mut first_crash: Option<(u32, Vec<u8>)> = None;

    for i in 0..config.max_iterations {
        let base = corpus.mutate_existing_input();
        let candidate = Mutator::apply(&base, mutation_strategy_for(i as usize));

        match run_target(&candidate) {
            Ok(edges) => {
                for edge in edges {
                    fuzzer.record_edge(edge);
                }
                corpus.add_corpus(&candidate);
            }
            Err(reason) => {
                fuzzer.record_crash();
                corpus.add_crash(&candidate);
                if first_crash.is_none() {
                    println!("crash found at iteration {i}: {reason} (input={candidate:?})");
                    first_crash = Some((i, candidate));
                }
            }
        }
    }

    fuzzer.update_coverage();
    println!("\niterations: {}", config.max_iterations);
    println!(
        "coverage: {}/{} edges ({:.1}%)",
        fuzzer.coverage,
        config.max_coverage,
        fuzzer.coverage_percent()
    );
    println!("crashes found: {}", fuzzer.crashes_found);
    println!("corpus size: {}", corpus.len());
    println!("crash corpus size: {}", corpus.crash_inputs.len());
}
