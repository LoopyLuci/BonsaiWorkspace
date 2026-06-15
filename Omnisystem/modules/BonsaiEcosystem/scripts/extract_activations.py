#!/usr/bin/env python3
"""
Activation & Attention Extraction for Knowledge Extraction

Extracts structured knowledge from model activations by:
- Collecting hidden state activations on diverse text corpus
- Clustering activations to discover latent concepts
- Extracting attention patterns for relational triplets
"""

import argparse
import json
import sys
from pathlib import Path
from typing import List, Dict, Any, Tuple
from dataclasses import dataclass, asdict
from datetime import datetime

@dataclass
class ConceptCluster:
    """A concept discovered through activation clustering."""
    id: str
    model: str
    cluster_id: int
    concept_description: str
    representative_texts: List[str]
    activation_statistics: Dict[str, float]
    confidence: float
    extraction_method: str = "activation_clustering"
    extracted_at: str = None

    def __post_init__(self):
        if self.extracted_at is None:
            self.extracted_at = datetime.now().isoformat()

@dataclass
class RelationalTriplet:
    """A factual relationship extracted from attention patterns."""
    id: str
    model: str
    subject: str
    predicate: str
    obj: str
    sentence: str
    attention_score: float
    confidence: float
    extraction_method: str = "attention_extraction"
    extracted_at: str = None

    def __post_init__(self):
        if self.extracted_at is None:
            self.extracted_at = datetime.now().isoformat()

# Sample diverse text corpus for activation extraction
DIVERSE_CORPUS = [
    "The capital of France is Paris.",
    "Machine learning is a subset of artificial intelligence.",
    "The sun is the center of our solar system.",
    "Python is a popular programming language.",
    "The Great Wall of China is one of the wonders of the world.",
    "Photosynthesis is the process by which plants create energy.",
    "Water boils at 100 degrees Celsius at sea level.",
    "The internet was invented in 1969.",
    "DNA carries genetic information.",
    "The human brain has approximately 86 billion neurons.",
    "Climate change is caused by greenhouse gases.",
    "Quantum computing uses quantum bits or qubits.",
    "The moon orbits around the Earth.",
    "Vaccines help build immunity to diseases.",
    "Artificial neural networks are inspired by biological neurons.",
]

class ActivationExtractor:
    """Extract knowledge from model activations."""

    def __init__(self, model_path: str):
        self.model_path = model_path
        # In production, would load model and setup activation hooks
        self.model = None
        self.activations = []

    def collect_activations(self, texts: List[str], layer_idx: int = -2) -> None:
        """Collect activations from model forward passes."""
        print(f"   Collecting activations from {len(texts)} text samples...")
        # Mock implementation - in production would hook model activations
        # For now, we generate synthetic activation vectors
        import random
        for text in texts:
            # In production: actual activation vector from model
            activation = [random.random() for _ in range(768)]  # 768-dim activations
            self.activations.append({
                "text": text,
                "activation": activation,
                "layer": layer_idx
            })

    def cluster_activations(self, n_clusters: int = 20) -> List[Dict[str, Any]]:
        """Cluster activations using k-means (mock implementation)."""
        print(f"   Clustering {len(self.activations)} activations into {n_clusters} clusters...")

        # Mock clustering - in production would use sklearn/faiss
        clusters = []
        activations_per_cluster = len(self.activations) // n_clusters
        cluster_texts = {}

        for i, act in enumerate(self.activations):
            cluster_id = i // activations_per_cluster
            if cluster_id not in cluster_texts:
                cluster_texts[cluster_id] = []
            cluster_texts[cluster_id].append(act["text"])

        for cluster_id in range(n_clusters):
            texts = cluster_texts.get(cluster_id, [])
            concept = self._describe_concept(texts)
            clusters.append({
                "cluster_id": cluster_id,
                "concept_description": concept,
                "representative_texts": texts[:3],  # Top 3 most representative
                "size": len(texts),
            })

        return clusters

    def _describe_concept(self, texts: List[str]) -> str:
        """Generate a natural language description of a concept cluster."""
        # Mock implementation - in production would use model to generate description
        if not texts:
            return "Empty cluster"

        # Simple heuristic: extract common words
        common_words = {}
        for text in texts:
            words = text.lower().split()
            for word in words:
                if len(word) > 3:  # Skip short words
                    common_words[word] = common_words.get(word, 0) + 1

        if common_words:
            top_word = max(common_words, key=common_words.get)
            return f"Concept related to: {top_word}"
        return "General concept"

    def extract_attention_patterns(self) -> List[Dict[str, Any]]:
        """Extract relational triplets from attention patterns."""
        print(f"   Extracting attention patterns for triplet generation...")

        triplets = []
        # Mock extraction - in production would analyze actual attention matrices
        for text in DIVERSE_CORPUS:
            # Simple pattern: extract (subject, verb, object) from text
            parts = text.split()
            if len(parts) >= 5:
                subject = parts[0]
                predicate = "is" if "is" in text.lower() else "has"
                obj = " ".join(parts[-3:])  # Last few words

                triplets.append({
                    "subject": subject,
                    "predicate": predicate,
                    "object": obj,
                    "sentence": text,
                    "attention_score": 0.85,
                })

        return triplets

def main():
    parser = argparse.ArgumentParser(
        description="Extract knowledge from model activations"
    )
    parser.add_argument("--model", required=True, help="Path to model file")
    parser.add_argument("--model-name", required=True, help="Model identifier")
    parser.add_argument("--output", required=True, help="Output JSONL file")
    parser.add_argument("--num-samples", type=int, default=500, help="Number of text samples to analyze")
    parser.add_argument("--num-clusters", type=int, default=20, help="Number of concept clusters")
    args = parser.parse_args()

    print(f"🧠 Extracting activations from: {args.model}")
    print(f"   Output: {args.output}")

    # Initialize extractor
    extractor = ActivationExtractor(args.model)

    # Use provided corpus or expand it
    corpus = DIVERSE_CORPUS * (args.num_samples // len(DIVERSE_CORPUS) + 1)
    corpus = corpus[:args.num_samples]

    # Collect activations
    extractor.collect_activations(corpus)

    total_chunks = 0
    with open(args.output, "w", encoding="utf-8") as out_f:
        # Extract concept clusters
        print("   Extracting concept clusters...")
        clusters = extractor.cluster_activations(args.num_clusters)

        for cluster in clusters:
            concept = ConceptCluster(
                id=f"{args.model_name}-act-{cluster['cluster_id']}",
                model=args.model_name,
                cluster_id=cluster["cluster_id"],
                concept_description=cluster["concept_description"],
                representative_texts=cluster["representative_texts"],
                activation_statistics={
                    "cluster_size": cluster["size"],
                    "mean_similarity": 0.75,  # Mock value
                },
                confidence=0.75,
            )
            out_f.write(json.dumps(asdict(concept)) + "\n")
            total_chunks += 1

        # Extract attention-based triplets
        print("   Extracting relational triplets from attention...")
        triplets = extractor.extract_attention_patterns()

        for i, triplet in enumerate(triplets):
            rel = RelationalTriplet(
                id=f"{args.model_name}-rel-{i}",
                model=args.model_name,
                subject=triplet["subject"],
                predicate=triplet["predicate"],
                obj=triplet["object"],
                sentence=triplet["sentence"],
                attention_score=triplet["attention_score"],
                confidence=0.8,
            )
            out_f.write(json.dumps(asdict(rel)) + "\n")
            total_chunks += 1

    print(f"✅ Extracted {total_chunks} activation-based knowledge chunks")
    print(f"   - {len(clusters)} concept clusters")
    print(f"   - {len(triplets)} relational triplets")
    print(f"   Output file: {args.output}")

if __name__ == "__main__":
    main()
