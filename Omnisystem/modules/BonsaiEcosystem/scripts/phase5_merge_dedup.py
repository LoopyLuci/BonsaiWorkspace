#!/usr/bin/env python3
"""
Phase 5: Merge, Deduplication & Quality Scoring
Consolidates all extractions, deduplicates, redacts PII, and scores quality
"""

import json
import os
import hashlib
import re
from pathlib import Path
from typing import List, Dict, Any, Set, Tuple

class KnowledgeDeduplicator:
    """Deduplicate and score knowledge chunks"""

    def __init__(self, quality_threshold: float = 0.6):
        self.quality_threshold = quality_threshold
        self.seen_hashes: Set[str] = set()
        self.pii_patterns = {
            "email": r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b",
            "phone": r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b",
            "ssn": r"\b\d{3}-\d{2}-\d{4}\b",
            "credit_card": r"\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b",
            "api_key": r"\b[A-Za-z0-9_-]{32,}\b",
        }

    def compute_hash(self, content: str) -> str:
        """Compute Blake3-equivalent hash (using SHA256 as fallback)"""
        return hashlib.sha256(content.encode()).hexdigest()

    def is_duplicate(self, content: str) -> bool:
        """Check if content has been seen before"""
        h = self.compute_hash(content)
        if h in self.seen_hashes:
            return True
        self.seen_hashes.add(h)
        return False

    def redact_pii(self, text: str) -> Tuple[str, List[str]]:
        """Redact personally identifiable information"""
        redacted = text
        pii_types = []

        for pii_type, pattern in self.pii_patterns.items():
            if re.search(pattern, redacted):
                pii_types.append(pii_type)
                redacted = re.sub(pattern, f"[REDACTED_{pii_type.upper()}]", redacted)

        return redacted, pii_types

    def score_quality(self, chunk: Dict[str, Any]) -> float:
        """Score chunk quality (0.0 to 1.0)"""
        score = 1.0

        # Extract content
        content = (
            chunk.get("answer") or
            chunk.get("response") or
            chunk.get("concept_description") or
            ""
        )

        # Length scoring
        if len(content) < 20:
            score *= 0.3
        elif len(content) > 5000:
            score *= 0.7

        # Generic response penalty
        generic_phrases = ["i'm not sure", "it depends", "unclear", "unknown"]
        if any(phrase in content.lower() for phrase in generic_phrases):
            score *= 0.8

        # Confidence-based scoring
        if "confidence" in chunk:
            score *= chunk["confidence"]

        return max(0.0, min(1.0, score))

    def process_chunks(self, chunks: List[Dict[str, Any]]) -> Tuple[List[Dict[str, Any]], Dict[str, int]]:
        """Process chunks: deduplicate, redact, score"""
        processed = []
        stats = {
            "total_input": len(chunks),
            "duplicates": 0,
            "pii_redacted": 0,
            "below_threshold": 0,
            "final_count": 0
        }

        for chunk in chunks:
            # Get content for dedup check
            content = (
                chunk.get("answer") or
                chunk.get("response") or
                chunk.get("concept_description") or
                ""
            )

            # Check for duplicate
            if self.is_duplicate(content):
                stats["duplicates"] += 1
                continue

            # Redact PII
            redacted_content, pii_types = self.redact_pii(content)
            if pii_types:
                stats["pii_redacted"] += 1
                # Update content in chunk
                if "answer" in chunk:
                    chunk["answer"] = redacted_content
                elif "response" in chunk:
                    chunk["response"] = redacted_content
                elif "concept_description" in chunk:
                    chunk["concept_description"] = redacted_content
                chunk["pii_redacted"] = pii_types

            # Score quality
            quality_score = self.score_quality(chunk)
            chunk["quality_score"] = quality_score

            # Filter by quality threshold
            if quality_score < self.quality_threshold:
                stats["below_threshold"] += 1
                continue

            # Add hash
            chunk["content_hash"] = self.compute_hash(content)
            processed.append(chunk)
            stats["final_count"] += 1

        return processed, stats

def main():
    input_dir = r"D:\Models\extracted_knowledge"
    output_file = os.path.join(input_dir, "merged_chunks.jsonl")

    print(f"\n🔗 PHASE 5: Merge, Deduplicate & Quality Scoring")
    print("=" * 70)

    # Load all extraction outputs
    all_chunks = []
    extraction_files = [
        os.path.join(input_dir, "extracted_synthetic_qa.jsonl"),
        os.path.join(input_dir, "extracted_activations.jsonl"),
        os.path.join(input_dir, "extracted_behavioral.jsonl"),
    ]

    print(f"\n📂 Loading extracted chunks...")
    for file in extraction_files:
        if os.path.exists(file):
            with open(file, 'r', encoding='utf-8') as f:
                count = 0
                for line in f:
                    try:
                        chunk = json.loads(line)
                        all_chunks.append(chunk)
                        count += 1
                    except json.JSONDecodeError:
                        pass
            print(f"   ✅ {os.path.basename(file):40} → {count:,} chunks")
        else:
            print(f"   ⚠️  {os.path.basename(file):40} → not found")

    print(f"\nTotal chunks loaded: {len(all_chunks):,}")

    if not all_chunks:
        print("\n⚠️  No chunks to process. Run Phase 2-4 first.")
        return 1

    # Process chunks
    deduplicator = KnowledgeDeduplicator(quality_threshold=0.6)
    print(f"\n📊 Processing chunks...")
    print(f"   - Checking for duplicates")
    print(f"   - Redacting PII")
    print(f"   - Computing quality scores")

    processed_chunks, stats = deduplicator.process_chunks(all_chunks)

    # Save merged chunks
    os.makedirs(os.path.dirname(output_file) or ".", exist_ok=True)
    with open(output_file, 'w', encoding='utf-8') as f:
        for chunk in processed_chunks:
            f.write(json.dumps(chunk) + '\n')

    # Summary
    print(f"\n✅ DEDUPLICATION COMPLETE")
    print("=" * 70)
    print(f"Input chunks:           {stats['total_input']:>10,}")
    print(f"Duplicates removed:     {stats['duplicates']:>10,} ({100*stats['duplicates']/max(1,stats['total_input']):>5.1f}%)")
    print(f"PII redacted:           {stats['pii_redacted']:>10,}")
    print(f"Below quality threshold: {stats['below_threshold']:>10,}")
    print(f"Output chunks:          {stats['final_count']:>10,} ({100*stats['final_count']/max(1,stats['total_input']):>5.1f}%)")

    dedup_ratio = 100 * (stats['total_input'] - stats['final_count']) / max(1, stats['total_input'])
    print(f"\nDeduplication ratio: {dedup_ratio:.1f}%")

    print(f"\n📁 Merged chunks saved to: {output_file}")
    print(f"\nNext step: Run Phase 6 (build KDB modules)")
    print(f"   python scripts/phase6_build_kdb.py")

    return 0

if __name__ == "__main__":
    exit(main())
