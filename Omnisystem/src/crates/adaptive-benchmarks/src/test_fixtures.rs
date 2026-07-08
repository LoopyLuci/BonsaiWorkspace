use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Test dataset with different sizes and domains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDataset {
    pub name: String,
    pub size: DatasetSize,
    pub domain: Domain,
    pub num_samples: usize,
    pub prompts: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DatasetSize {
    Small,      // 100 samples
    Medium,     // 1000 samples
    Large,      // 10K samples
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Domain {
    GeneralKnowledge,
    Code,
    Math,
    Reasoning,
    Creative,
}

impl TestDataset {
    pub fn small_general() -> Self {
        Self {
            name: "small_general_knowledge".to_string(),
            size: DatasetSize::Small,
            domain: Domain::GeneralKnowledge,
            num_samples: 100,
            prompts: Self::generate_general_prompts(100),
        }
    }

    pub fn medium_code() -> Self {
        Self {
            name: "medium_code".to_string(),
            size: DatasetSize::Medium,
            domain: Domain::Code,
            num_samples: 1000,
            prompts: Self::generate_code_prompts(1000),
        }
    }

    pub fn large_reasoning() -> Self {
        Self {
            name: "large_reasoning".to_string(),
            size: DatasetSize::Large,
            domain: Domain::Reasoning,
            num_samples: 10000,
            prompts: Self::generate_reasoning_prompts(10000),
        }
    }

    pub fn wikitext_validation() -> Self {
        Self {
            name: "wikitext_validation".to_string(),
            size: DatasetSize::Medium,
            domain: Domain::GeneralKnowledge,
            num_samples: 1000,
            prompts: Self::generate_wiki_prompts(1000),
        }
    }

    pub fn openwebtext_validation() -> Self {
        Self {
            name: "openwebtext_validation".to_string(),
            size: DatasetSize::Medium,
            domain: Domain::GeneralKnowledge,
            num_samples: 1000,
            prompts: Self::generate_web_prompts(1000),
        }
    }

    pub fn humaneval_benchmark() -> Self {
        Self {
            name: "humaneval_benchmark".to_string(),
            size: DatasetSize::Small,
            domain: Domain::Code,
            num_samples: 164,  // Standard HumanEval has 164 problems
            prompts: Self::generate_humaneval_prompts(164),
        }
    }

    pub fn mmlu_benchmark() -> Self {
        Self {
            name: "mmlu_benchmark".to_string(),
            size: DatasetSize::Medium,
            domain: Domain::GeneralKnowledge,
            num_samples: 14000,  // Approximate MMLU test set
            prompts: Self::generate_mmlu_prompts(100),  // Sample for testing
        }
    }

    fn generate_general_prompts(count: usize) -> Vec<String> {
        (0..count).map(|i| {
            match i % 5 {
                0 => "What is machine learning?".to_string(),
                1 => "Explain quantum computing in simple terms.".to_string(),
                2 => "How does photosynthesis work?".to_string(),
                3 => "What are the benefits of renewable energy?".to_string(),
                _ => "Describe the water cycle.".to_string(),
            }
        }).collect()
    }

    fn generate_code_prompts(count: usize) -> Vec<String> {
        (0..count).map(|i| {
            match i % 5 {
                0 => "Write a Python function to calculate fibonacci numbers.".to_string(),
                1 => "Create a Rust function to parse JSON.".to_string(),
                2 => "Implement a binary search in JavaScript.".to_string(),
                3 => "Write a Python class for a linked list.".to_string(),
                _ => "Create a function to sort an array in C++.".to_string(),
            }
        }).collect()
    }

    fn generate_math_prompts(count: usize) -> Vec<String> {
        (0..count).map(|i| {
            match i % 5 {
                0 => "Solve: 2x + 5 = 13".to_string(),
                1 => "What is the derivative of x^3?".to_string(),
                2 => "Calculate the area of a circle with radius 5.".to_string(),
                3 => "What is 20% of 150?".to_string(),
                _ => "Find the roots of x^2 - 5x + 6 = 0.".to_string(),
            }
        }).collect()
    }

    fn generate_reasoning_prompts(count: usize) -> Vec<String> {
        (0..count).map(|i| {
            match i % 5 {
                0 => "If all roses are flowers and all flowers are plants, are all roses plants?".to_string(),
                1 => "John is taller than Mary. Mary is taller than Susan. Who is the tallest?".to_string(),
                2 => "If a plane crashes on the border between France and Germany, where do they bury the survivors?".to_string(),
                3 => "What letter comes next in this sequence: A, C, E, G, ?".to_string(),
                _ => "If you have 10 apples and give away 3, how many do you have left?".to_string(),
            }
        }).collect()
    }

    fn generate_creative_prompts(count: usize) -> Vec<String> {
        (0..count).map(|i| {
            match i % 3 {
                0 => "Write a short poem about spring.".to_string(),
                1 => "Create a story opening: 'The old lighthouse keeper had a secret...'".to_string(),
                _ => "Write a dialogue between two characters meeting for the first time.".to_string(),
            }
        }).collect()
    }

    fn generate_wiki_prompts(count: usize) -> Vec<String> {
        (0..count).map(|i| {
            match i % 3 {
                0 => "article about the history of computing".to_string(),
                1 => "information on climate change impacts".to_string(),
                _ => "overview of space exploration missions".to_string(),
            }
        }).collect()
    }

    fn generate_web_prompts(count: usize) -> Vec<String> {
        (0..count).map(|i| {
            match i % 4 {
                0 => "blog post about productivity tips".to_string(),
                1 => "news article on technology trends".to_string(),
                2 => "tutorial on web development".to_string(),
                _ => "review of a new product".to_string(),
            }
        }).collect()
    }

    fn generate_humaneval_prompts(count: usize) -> Vec<String> {
        (0..count).map(|i| {
            match i % 3 {
                0 => format!("Write a function to solve problem {}", i),
                1 => format!("Complete the code for exercise {}", i),
                _ => format!("Implement function for task {}", i),
            }
        }).collect()
    }

    fn generate_mmlu_prompts(count: usize) -> Vec<String> {
        (0..count).map(|i| {
            match i % 4 {
                0 => "Multiple choice question on physics.".to_string(),
                1 => "Question testing historical knowledge.".to_string(),
                2 => "Chemistry concept question.".to_string(),
                _ => "Biology examination question.".to_string(),
            }
        }).collect()
    }
}

/// Test fixture for reproducible testing
#[derive(Debug, Clone)]
pub struct TestFixture {
    pub name: String,
    pub datasets: Vec<TestDataset>,
    pub random_seed: u64,
}

impl TestFixture {
    pub fn new(name: String, seed: u64) -> Self {
        Self {
            name,
            datasets: Vec::new(),
            random_seed: seed,
        }
    }

    pub fn with_all_datasets() -> Self {
        let mut fixture = Self::new("comprehensive_fixture".to_string(), 42);
        fixture.datasets.push(TestDataset::small_general());
        fixture.datasets.push(TestDataset::medium_code());
        fixture.datasets.push(TestDataset::wikitext_validation());
        fixture.datasets.push(TestDataset::humaneval_benchmark());
        fixture.datasets.push(TestDataset::mmlu_benchmark());
        fixture
    }

    pub fn benchmark_suite() -> Self {
        let mut fixture = Self::new("benchmark_suite".to_string(), 42);
        fixture.datasets.push(TestDataset::wikitext_validation());
        fixture.datasets.push(TestDataset::humaneval_benchmark());
        fixture.datasets.push(TestDataset::mmlu_benchmark());
        fixture
    }

    pub fn quick_test_suite() -> Self {
        let mut fixture = Self::new("quick_test".to_string(), 42);
        fixture.datasets.push(TestDataset::small_general());
        fixture
    }

    pub fn total_samples(&self) -> usize {
        self.datasets.iter().map(|d| d.num_samples).sum()
    }

    pub fn get_dataset(&self, domain: Domain) -> Option<&TestDataset> {
        self.datasets.iter().find(|d| d.domain == domain)
    }
}

/// Deterministic random prompt generation
pub fn generate_deterministic_prompts(seed: u64, count: usize) -> Vec<String> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut prompts = Vec::new();
    let templates = vec![
        "Question: {}",
        "Complete the following: {}",
        "Explain: {}",
        "Analyze: {}",
        "Summarize: {}",
    ];

    for i in 0..count {
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        i.hash(&mut hasher);
        let _hash = hasher.finish();

        let template_idx = (i * 7) % templates.len();
        let content = format!("concept_{}", i);

        prompts.push(format!(templates[template_idx], content));
    }

    prompts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dataset_creation() {
        let dataset = TestDataset::small_general();
        assert_eq!(dataset.num_samples, 100);
        assert_eq!(dataset.prompts.len(), 100);
    }

    #[test]
    fn test_dataset_sizes() {
        let small = TestDataset::small_general();
        let medium = TestDataset::medium_code();
        let large = TestDataset::large_reasoning();

        assert!(small.prompts.len() < medium.prompts.len());
        assert!(medium.prompts.len() < large.prompts.len());
    }

    #[test]
    fn test_benchmark_datasets() {
        let humaneval = TestDataset::humaneval_benchmark();
        assert_eq!(humaneval.num_samples, 164);

        let mmlu = TestDataset::mmlu_benchmark();
        assert!(!mmlu.prompts.is_empty());
    }

    #[test]
    fn test_fixture_creation() {
        let fixture = TestFixture::with_all_datasets();
        assert!(!fixture.datasets.is_empty());
        assert_eq!(fixture.random_seed, 42);
    }

    #[test]
    fn test_fixture_total_samples() {
        let fixture = TestFixture::quick_test_suite();
        assert!(fixture.total_samples() > 0);
    }

    #[test]
    fn test_deterministic_prompts() {
        let prompts1 = generate_deterministic_prompts(42, 10);
        let prompts2 = generate_deterministic_prompts(42, 10);

        assert_eq!(prompts1, prompts2);
    }

    #[test]
    fn test_get_dataset_by_domain() {
        let fixture = TestFixture::with_all_datasets();
        assert!(fixture.get_dataset(Domain::Code).is_some());
        assert!(fixture.get_dataset(Domain::GeneralKnowledge).is_some());
    }
}
