#!/usr/bin/env python3
"""
Generate sample KDB modules for demonstration
Shows what the extraction pipeline produces
"""

import json
import zipfile
import os
from datetime import datetime

def create_sample_module(model_name: str, output_dir: str):
    """Create a sample KDB module for demonstration"""
    os.makedirs(output_dir, exist_ok=True)

    output_file = os.path.join(output_dir, f"{model_name}.kmod")

    # Sample chunks (demonstrating what extraction produces)
    sample_chunks = [
        {
            "id": f"{model_name}-qa-science-001",
            "model": model_name,
            "domain": "science",
            "difficulty": "medium",
            "question": "Explain photosynthesis",
            "answer": "Photosynthesis is the process by which green plants and some other organisms use sunlight to synthesize nutrients from carbon dioxide and water. It is the foundation of most life on Earth. In plants, photosynthesis typically occurs in the leaves and involves the conversion of light energy into chemical energy stored in glucose. The process consists of two main stages: the light-dependent reactions that occur in the thylakoid membrane, and the light-independent reactions (Calvin cycle) that occur in the stroma.",
            "confidence": 0.92,
            "extraction_method": "synthetic_qa",
            "quality_score": 0.88,
            "content_hash": "a1b2c3d4e5f6g7h8",
            "extracted_at": datetime.now().isoformat()
        },
        {
            "id": f"{model_name}-qa-science-002",
            "model": model_name,
            "domain": "science",
            "difficulty": "hard",
            "question": "What is DNA and how does it work?",
            "answer": "DNA (deoxyribonucleic acid) is a molecule that encodes genetic instructions for life. Each DNA molecule consists of two strands forming a double helix structure, made up of nucleotides containing deoxyribose sugar, phosphate, and nitrogenous bases (adenine, thymine, guanine, cytosine). The bases pair specifically: adenine with thymine, and guanine with cytosine. DNA stores information in the sequence of these base pairs and replicates itself before cell division through a process involving DNA polymerase enzymes. The information in DNA is transcribed to RNA and then translated into proteins, which perform most cellular functions.",
            "confidence": 0.90,
            "extraction_method": "synthetic_qa",
            "quality_score": 0.87,
            "content_hash": "b2c3d4e5f6g7h8i9",
            "extracted_at": datetime.now().isoformat()
        },
        {
            "id": f"{model_name}-qa-programming-001",
            "model": model_name,
            "domain": "programming",
            "difficulty": "medium",
            "question": "What is object-oriented programming?",
            "answer": "Object-oriented programming (OOP) is a programming paradigm that organizes code into objects, which are instances of classes. Classes define the structure (properties/attributes) and behavior (methods) of objects. Key principles include: Encapsulation (bundling data and methods), Inheritance (creating hierarchies of classes), Polymorphism (objects responding to the same message in different ways), and Abstraction (hiding complex implementation details). OOP promotes code reusability, modularity, and easier maintenance through inheritance and composition.",
            "confidence": 0.89,
            "extraction_method": "synthetic_qa",
            "quality_score": 0.86,
            "content_hash": "c3d4e5f6g7h8i9j0",
            "extracted_at": datetime.now().isoformat()
        },
        {
            "id": f"{model_name}-act-001",
            "model": model_name,
            "type": "activation_cluster",
            "cluster_id": 0,
            "concept_description": "Biological processes and life sciences knowledge representation",
            "representative_texts": [
                "Cells are the basic unit of life",
                "Organisms grow and reproduce",
                "Energy flows through living systems"
            ],
            "confidence": 0.75,
            "extraction_method": "activation_clustering",
            "quality_score": 0.80,
            "content_hash": "d4e5f6g7h8i9j0k1",
            "extracted_at": datetime.now().isoformat()
        },
        {
            "id": f"{model_name}-beh-001",
            "model": model_name,
            "scenario_type": "open_conversation",
            "prompt": "Tell me about your strengths and limitations",
            "response": "I am a language model trained to understand and generate human language. My strengths include broad knowledge across many domains, ability to explain complex topics clearly, and consistency in following instructions. However, I have limitations: I cannot access real-time information, I may make mistakes or hallucinate facts, I have a knowledge cutoff date, and I process information statically without true understanding. I work best when given clear, specific instructions and context.",
            "response_length": 65,
            "refusal_detected": False,
            "confidence_level": "high",
            "tone": "formal",
            "domain_relevance": ["general", "meta"],
            "extraction_method": "behavioral_scenario",
            "quality_score": 0.85,
            "content_hash": "e5f6g7h8i9j0k1l2",
            "extracted_at": datetime.now().isoformat()
        }
    ]

    # Metadata
    metadata = {
        "name": f"{model_name}-knowledge",
        "version": "1.0.0",
        "source_model": model_name,
        "num_chunks": len(sample_chunks),
        "mean_quality_score": 0.85,
        "extraction_date": datetime.now().isoformat(),
        "domains": ["science", "programming", "mathematics", "general"],
        "extraction_methods": ["synthetic_qa", "activation_clustering", "behavioral_scenario"],
        "total_tokens": 2847,
        "created_with": "Bonsai Knowledge Extraction Fabric (KEF)",
    }

    # Create .kmod (ZIP archive)
    with zipfile.ZipFile(output_file, 'w', zipfile.ZIP_DEFLATED) as kmod:
        # Metadata
        kmod.writestr("metadata.json", json.dumps(metadata, indent=2))

        # Chunks
        chunks_jsonl = "\n".join(json.dumps(c) for c in sample_chunks)
        kmod.writestr("chunks.jsonl", chunks_jsonl)

        # Index metadata
        index_meta = {
            "type": "hnsw",
            "dimension": 384,
            "num_vectors": len(sample_chunks),
            "note": "HNSW index for semantic search"
        }
        kmod.writestr("index_meta.json", json.dumps(index_meta, indent=2))

        # README
        readme = f"""# {metadata['name']}

## Sample KDB Module

This is a sample module generated by the Bonsai Knowledge Extraction Fabric.

## Statistics
- Chunks: {metadata['num_chunks']}
- Quality: {metadata['mean_quality_score']:.3f}/1.0
- Tokens: {metadata['total_tokens']:,}

## Domains
{', '.join(metadata['domains'])}

## Extraction Methods
{', '.join(metadata['extraction_methods'])}

---
Created: {metadata['extraction_date']}
"""
        kmod.writestr("README.md", readme)

    return output_file

def main():
    output_dir = r"Z:\Projects\BonsaiWorkspace\kdb-modules"

    print("📦 Generating sample KDB modules for demonstration...\n")

    models = [
        "tinyllama-1.1b",
        "llama-2-7b",
        "mistral-7b",
    ]

    for model in models:
        output_file = create_sample_module(model, output_dir)
        file_size_kb = os.path.getsize(output_file) / 1024
        print(f"✅ {model:20} → {os.path.basename(output_file)} ({file_size_kb:.1f} KB)")

    print(f"\n📁 Sample modules created in: {output_dir}")
    print(f"\n🔍 You can inspect them:")
    print(f"   # List modules")
    print(f"   Get-ChildItem {output_dir}\\*.kmod")
    print(f"\n   # Extract and view chunks")
    print(f"   $zip = [System.IO.Compression.ZipFile]::OpenRead('{output_dir}\\tinyllama-1.1b.kmod')")
    print(f"   $zip.Entries | Select-Object Name")

if __name__ == "__main__":
    main()
