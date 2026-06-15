#!/usr/bin/env python3
"""
Merge & Deduplication Pipeline

Consolidates extracted knowledge from multiple methods and models,
deduplicates by content hash, applies quality scoring, and redacts PII.
"""

import argparse
import json
import hashlib
from pathlib import Path
from typing import List, Dict, Any, Set
from dataclasses import dataclass, asdict
from datetime import datetime
import re

@dataclass
class DedupStats:
    """Statistics about deduplication process."""
    total_input: int
    total_unique: int
    duplicates_removed: int
    pii_redacted: int
    below_quality_threshold: int
    final_count: int

class ContentDeduplicator:
    """Deduplicate knowledge chunks by content hash."""

    def __init__(self, method: str = "blake3"):
        self.method = method
        self.seen_hashes: Set[str] = set()
        self.duplicate_count = 0

    def hash_content(self, content: str) -> str:
        """Compute hash of content."""
        if self.method == "blake3":
            # Fallback to sha256 if blake3 not available
            return hashlib.sha256(content.encode()).hexdigest()
        else:
            return hashlib.sha256(content.encode()).hexdigest()

    def is_duplicate(self, content: str) -> bool:
        """Check if content hash has been seen."""
        h = self.hash_content(content)
        if h in self.seen_hashes:
            self.duplicate_count += 1
            return True
        self.seen_hashes.add(h)
        return False

class PIIRedactor:
    """Redact personally identifiable information from chunks."""

    # Patterns for common PII
    PATTERNS = {
        "email": r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b",
        "phone": r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b",
        "ssn": r"\b\d{3}-\d{2}-\d{4}\b",
        "credit_card": r"\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b",
        "ipv4": r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b",
        "api_key": r"\b[A-Za-z0-9_-]{32,}\b",
    }

    def __init__(self):
        self.patterns = {name: re.compile(pattern) for name, pattern in self.PATTERNS.items()}

    def redact(self, text: str) -> tuple[str, List[str]]:
        """Redact PII from text, return (redacted_text, pii_types_found)."""
        redacted_text = text
        pii_types_found = []

        for pii_type, pattern in self.patterns.items():
            matches = pattern.findall(redacted_text)
            if matches:
                pii_types_found.append(pii_type)
                redacted_text = pattern.sub(f"[REDACTED_{pii_type.upper()}]", redacted_text)

        return redacted_text, pii_types_found

class QualityScorer:
    """Score quality of extracted knowledge chunks."""

    def __init__(self):
        # Simple heuristic-based scoring (in production, use ML model)
        self.min_response_length = 10
        self.max_response_length = 10000

    def score(self, chunk: Dict[str, Any]) -> float:
        """Score a chunk on quality (0.0 to 1.0)."""
        score = 1.0

        # Extract relevant field based on chunk type
        content = (
            chunk.get("answer")
            or chunk.get("response")
            or chunk.get("concept_description")
            or ""
        )

        # Check length
        if len(content) < self.min_response_length:
            score *= 0.3  # Very short responses score low
        elif len(content) > self.max_response_length:
            score *= 0.7  # Very long responses might be overfit

        # Check for generic responses
        generic_phrases = ["i'm not sure", "it depends", "unclear"]
        if any(phrase in content.lower() for phrase in generic_phrases):
            score *= 0.8

        # Check confidence field if present
        if "confidence" in chunk:
            score *= chunk["confidence"]

        return max(0.0, min(1.0, score))

def merge_jsonl_files(input_files: List[Path]) -> List[Dict[str, Any]]:
    """Merge JSONL files into a single list."""
    chunks = []
    for input_file in input_files:
        if not input_file.exists():
            print(f"   ⚠️  File not found: {input_file}")
            continue

        with open(input_file, "r", encoding="utf-8") as f:
            for line_num, line in enumerate(f, 1):
                try:
                    chunk = json.loads(line)
                    chunks.append(chunk)
                except json.JSONDecodeError as e:
                    print(f"   ⚠️  Invalid JSON in {input_file}:{line_num}: {e}")

    return chunks

def main():
    parser = argparse.ArgumentParser(
        description="Merge extraction outputs and deduplicate"
    )
    parser.add_argument("--inputs", nargs="+", required=True, help="Input JSONL files")
    parser.add_argument("--output", required=True, help="Output merged JSONL file")
    parser.add_argument("--quality-threshold", type=float, default=0.6, help="Minimum quality score")
    args = parser.parse_args()

    print(f"🔗 Merging and deduplicating knowledge chunks...")
    print(f"   Output: {args.output}")

    # Parse input files (handle glob patterns)
    input_files = []
    for input_spec in args.inputs:
        path = Path(input_spec)
        if "*" in input_spec:
            input_files.extend(Path(".").glob(input_spec))
        else:
            input_files.append(path)

    # Merge files
    print(f"   Reading from {len(input_files)} files...")
    chunks = merge_jsonl_files(input_files)
    print(f"   Loaded {len(chunks)} total chunks")

    # Initialize processors
    deduplicator = ContentDeduplicator()
    redactor = PIIRedactor()
    scorer = QualityScorer()

    # Process chunks
    unique_chunks = []
    pii_count = 0
    low_quality_count = 0

    print(f"   Processing chunks...")
    for i, chunk in enumerate(chunks):
        if (i + 1) % 1000 == 0:
            print(f"     {i+1}/{len(chunks)} processed...")

        # Determine content field
        content = (
            chunk.get("answer")
            or chunk.get("response")
            or chunk.get("concept_description")
            or ""
        )

        # Skip if duplicate
        if deduplicator.is_duplicate(content):
            continue

        # Redact PII
        redacted_content, pii_types = redactor.redact(content)
        if pii_types:
            pii_count += 1
            if "answer" in chunk:
                chunk["answer"] = redacted_content
            elif "response" in chunk:
                chunk["response"] = redacted_content
            elif "concept_description" in chunk:
                chunk["concept_description"] = redacted_content
            chunk["pii_redacted"] = pii_types

        # Score quality
        quality_score = scorer.score(chunk)
        chunk["quality_score"] = quality_score

        # Filter by quality threshold
        if quality_score < args.quality_threshold:
            low_quality_count += 1
            continue

        # Add hash
        chunk["content_hash"] = deduplicator.hash_content(content)
        unique_chunks.append(chunk)

    # Write deduplicated output
    with open(args.output, "w", encoding="utf-8") as f:
        for chunk in unique_chunks:
            f.write(json.dumps(chunk) + "\n")

    # Statistics
    stats = DedupStats(
        total_input=len(chunks),
        total_unique=len(unique_chunks),
        duplicates_removed=deduplicator.duplicate_count,
        pii_redacted=pii_count,
        below_quality_threshold=low_quality_count,
        final_count=len(unique_chunks),
    )

    print(f"\n📊 Deduplication Statistics:")
    print(f"   Total input chunks: {stats.total_input}")
    print(f"   Duplicates removed: {stats.duplicates_removed} ({100*stats.duplicates_removed/max(1,stats.total_input):.1f}%)")
    print(f"   PII redacted chunks: {stats.pii_redacted}")
    print(f"   Below quality threshold: {stats.low_quality_count}")
    print(f"   Final unique chunks: {stats.final_count}")
    print(f"   Dedup ratio: {(stats.total_input - stats.final_count) / max(1, stats.total_input) * 100:.1f}%")

    print(f"\n✅ Merged and deduplicated chunks written to: {args.output}")

if __name__ == "__main__":
    main()
