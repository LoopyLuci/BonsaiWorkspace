//! Context assembly for AI models
//!
//! Assembles retrieved atoms into a coherent, optimized context for LLMs.

use crate::error::Result;
use crate::retrieval::QueryResult;

/// Assemble context hierarchically
///
/// Takes query results and produces a context string suitable for feeding
/// to an LLM, with markers indicating source, resolution, and freshness.
pub async fn assemble_hierarchical(results: &QueryResult, max_tokens: usize) -> Result<String> {
    let mut context = String::new();
    let mut token_count = 0;

    // Sort atoms by relevance (score)
    let mut atoms = results.atoms.clone();
    atoms.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    for atom in atoms {
        let text = atom
            .atom
            .full_text()
            .unwrap_or(&atom.atom.resolutions[atom.resolution as usize].text);

        let text_tokens = text.len() / 4; // Rough estimate

        if token_count + text_tokens > max_tokens {
            // Too long - might use lower resolution for remaining atoms
            break;
        }

        // Add marker with metadata
        let marker = format!(
            "\n[{} – score:{:.2}% – {}]\n",
            atom.atom.metadata.source, atom.score * 100.0, atom.resolution
        );

        context.push_str(&marker);
        context.push_str(text);
        context.push_str("\n");

        token_count += text_tokens;
    }

    Ok(context)
}

/// Compress context by using lower resolution for older atoms
pub fn compress_context(context: &str, max_tokens: usize) -> Result<String> {
    // Simple truncation for MVP
    if context.len() / 4 > max_tokens {
        Ok(context[0..max_tokens * 4].to_string())
    } else {
        Ok(context.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atom::{AtomMetadata, SemanticAtom, SourceType};
    use crate::retrieval::RetrievedAtom;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_assemble_hierarchical() {
        let atom = SemanticAtom::from_text(
            "Test content".to_string(),
            AtomMetadata {
                source: SourceType::UserInput,
                agent_id: Uuid::nil(),
                conversation_id: None,
                tags: vec![],
                importance: 1.0,
            },
            3,
        )
        .unwrap();

        let results = QueryResult {
            atoms: vec![RetrievedAtom {
                atom,
                score: 0.95,
                resolution: crate::atom::ResolutionLevel::Full,
            }],
            latency_us: 100,
        };

        let context = assemble_hierarchical(&results, 1000).await.unwrap();
        assert!(context.contains("Test content"));
    }
}
