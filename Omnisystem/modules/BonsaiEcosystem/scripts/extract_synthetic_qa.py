#!/usr/bin/env python3
"""
Synthetic Question-Answer Generation for Knowledge Extraction

Generates diverse questions across multiple domains and queries a model
to extract its knowledge in the form of (question, answer) pairs.
"""

import argparse
import json
import sys
from pathlib import Path
from typing import List, Dict, Any
from dataclasses import dataclass, asdict
from datetime import datetime

@dataclass
class QAPair:
    """A question-answer pair extracted from a model."""
    id: str
    model: str
    domain: str
    difficulty: str
    question: str
    answer: str
    confidence: float
    extraction_method: str = "synthetic_qa"
    extracted_at: str = None

    def __post_init__(self):
        if self.extracted_at is None:
            self.extracted_at = datetime.now().isoformat()

# Domain taxonomy with example questions
DOMAIN_TAXONOMY = {
    "science": [
        "Explain photosynthesis",
        "What is the law of conservation of energy?",
        "Describe the water cycle",
        "What is DNA?",
        "Explain evolution",
        "What are the states of matter?",
        "Describe the structure of an atom",
        "What is cell division?",
        "Explain gravity",
        "What is thermodynamics?",
    ],
    "mathematics": [
        "What is the Pythagorean theorem?",
        "Explain linear regression",
        "What is calculus?",
        "Describe prime numbers",
        "What is probability?",
        "Explain algebra",
        "What is geometry?",
        "Describe statistics",
        "What is a derivative?",
        "Explain matrices",
    ],
    "history": [
        "What caused World War II?",
        "Describe the Industrial Revolution",
        "What was the Renaissance?",
        "Explain the fall of the Roman Empire",
        "What is the Enlightenment?",
        "Describe colonialism",
        "What was the American Revolution?",
        "Explain feudalism",
        "What is the Great Depression?",
        "Describe the Cold War",
    ],
    "geography": [
        "What is the capital of France?",
        "Describe the Sahara Desert",
        "What is the highest mountain on Earth?",
        "Explain plate tectonics",
        "What are the continents?",
        "Describe the Amazon Rainforest",
        "What is climate change?",
        "Explain weather patterns",
        "What are time zones?",
        "Describe ocean currents",
    ],
    "technology": [
        "What is artificial intelligence?",
        "Explain machine learning",
        "What is cloud computing?",
        "Describe blockchain",
        "What is the internet?",
        "Explain quantum computing",
        "What is cybersecurity?",
        "Describe 5G networks",
        "What is big data?",
        "Explain neural networks",
    ],
    "programming": [
        "What is a programming language?",
        "Explain object-oriented programming",
        "What is a data structure?",
        "Describe algorithms",
        "What is version control?",
        "Explain web development",
        "What is a database?",
        "Describe API design",
        "What is testing?",
        "Explain concurrency",
    ],
    "arts": [
        "What is perspective in art?",
        "Describe impressionism",
        "What is composition?",
        "Explain color theory",
        "What is surrealism?",
        "Describe cubism",
        "What is abstraction?",
        "Explain Renaissance art",
        "What is sculpture?",
        "Describe portrait painting",
    ],
    "literature": [
        "What makes a good story?",
        "Explain narrative structure",
        "What is imagery?",
        "Describe symbolism",
        "What is a metaphor?",
        "Explain foreshadowing",
        "What is theme?",
        "Describe character development",
        "What is dialogue?",
        "Explain point of view",
    ],
    "philosophy": [
        "What is ethics?",
        "Explain epistemology",
        "What is metaphysics?",
        "Describe existentialism",
        "What is stoicism?",
        "Explain determinism vs free will?",
        "What is skepticism?",
        "Describe virtue ethics",
        "What is logic?",
        "Explain phenomenology",
    ],
    "health": [
        "What is nutrition?",
        "Explain the immune system",
        "What is metabolism?",
        "Describe mental health",
        "What is exercise physiology?",
        "Explain vaccines",
        "What is sleep?",
        "Describe hormones",
        "What is aging?",
        "Explain inflammation",
    ],
}

class QuestionGenerator:
    """Generate diverse questions for knowledge extraction."""

    def __init__(self, seed: int = 42):
        self.seed = seed
        import random
        random.seed(seed)

    def generate_questions(self, domain: str, count: int, difficulty_distribution: Dict[str, float] = None) -> List[str]:
        """Generate questions for a domain."""
        if difficulty_distribution is None:
            difficulty_distribution = {"easy": 0.4, "medium": 0.4, "hard": 0.2}

        base_questions = DOMAIN_TAXONOMY.get(domain, [])
        if not base_questions:
            return []

        # For this demo, just return base questions expanded
        questions = []
        for base_q in base_questions:
            questions.append(base_q)
            questions.append(f"Explain {base_q.lower()}")
            questions.append(f"What is {base_q.lower()}?")
            if len(questions) >= count:
                return questions[:count]

        return questions[:count]

    def all_domains(self) -> List[str]:
        """Return all available domains."""
        return list(DOMAIN_TAXONOMY.keys())

class ModelQuerier:
    """Query a model for answers (mock implementation for now)."""

    def __init__(self, model_path: str, temperature: float = 0.0):
        self.model_path = model_path
        self.temperature = temperature
        # In production, would load actual model here
        self.model = None

    def query(self, question: str) -> str:
        """Query the model with a question."""
        # Mock implementation - in production, would call actual model inference
        # This is a placeholder that returns a generic response
        return f"This is a sample answer to: '{question}'. In production, the model would be queried here."

    def batch_query(self, questions: List[str], batch_size: int = 10) -> List[str]:
        """Query multiple questions in batches."""
        answers = []
        for i in range(0, len(questions), batch_size):
            batch = questions[i:i+batch_size]
            for q in batch:
                answers.append(self.query(q))
        return answers

def main():
    parser = argparse.ArgumentParser(
        description="Extract knowledge from a model via synthetic question generation"
    )
    parser.add_argument("--model", required=True, help="Path to model file")
    parser.add_argument("--model-name", required=True, help="Model identifier")
    parser.add_argument("--num-questions", type=int, default=100, help="Number of questions per domain")
    parser.add_argument("--output", required=True, help="Output JSONL file")
    parser.add_argument("--temperature", type=float, default=0.0, help="Sampling temperature")
    parser.add_argument("--batch-size", type=int, default=10, help="Batch size for inference")
    args = parser.parse_args()

    print(f"🧠 Extracting synthetic Q&A from: {args.model}")
    print(f"   Output: {args.output}")

    # Generate questions
    generator = QuestionGenerator()
    domains = generator.all_domains()

    total_questions = 0
    total_answers = 0

    with open(args.output, "w", encoding="utf-8") as out_f:
        for domain in domains:
            questions = generator.generate_questions(domain, args.num_questions)
            print(f"   📝 {domain.capitalize()}: {len(questions)} questions")

            # Mock model querying (in production, would use actual model)
            querier = ModelQuerier(args.model, temperature=args.temperature)
            answers = querier.batch_query(questions)

            # Write Q&A pairs
            for q, a in zip(questions, answers):
                qa = QAPair(
                    id=f"{args.model_name}-qa-{total_answers}",
                    model=args.model_name,
                    domain=domain,
                    difficulty="medium",  # Placeholder
                    question=q,
                    answer=a,
                    confidence=0.85,  # Placeholder
                )
                out_f.write(json.dumps(asdict(qa)) + "\n")
                total_answers += 1

            total_questions += len(questions)

    print(f"✅ Extracted {total_answers} Q&A pairs ({total_questions} questions)")
    print(f"   Domains covered: {', '.join(domains)}")
    print(f"   Output file: {args.output}")

if __name__ == "__main__":
    main()
