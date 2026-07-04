//! Task Catalog — 50 production-grade distributed computing task profiles.
//!
//! Each [`TaskProfile`] is a registered, schedulable workload the Bonsai
//! Compute Fabric knows how to distribute. Profiles describe the scheduling
//! topology, resource shape, data volume, and URV cost band so the
//! `CoordinatorActor` can match a task to capable nodes and the marketplace
//! can price it. Eight profiles are *Bonsai-native* — they make the ecosystem
//! improve itself (F³ fuzzing, Survival rule synthesis, scheduler evolution,
//! causal fault localisation, agent evolution, KDB indexing, proof search).

use crate::types::{FabricTask, TaskType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Taxonomy ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskCategory {
    AiMl,
    BuildCi,
    Multimedia,
    Simulation,
    DataAnalytics,
    Cryptography,
    Security,
    EdgeIot,
    BonsaiNative,
    Interop,
}

impl TaskCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskCategory::AiMl => "AI & Machine Learning",
            TaskCategory::BuildCi => "Software Build & CI/CD",
            TaskCategory::Multimedia => "Multimedia & Rendering",
            TaskCategory::Simulation => "Scientific & Engineering Simulation",
            TaskCategory::DataAnalytics => "Data Processing & Analytics",
            TaskCategory::Cryptography => "Cryptography & Privacy",
            TaskCategory::Security => "Security & Program Analysis",
            TaskCategory::EdgeIot => "Real-Time Edge & IoT",
            TaskCategory::BonsaiNative => "Bonsai Self-Improvement",
            TaskCategory::Interop => "Interoperability & Volunteer Computing",
        }
    }
}

/// How the fabric decomposes and coordinates the workload across nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulingStrategy {
    EmbarrassinglyParallel,
    MapReduce,
    AllReduce,
    ParameterServer,
    PipelineParallel,
    TensorParallel,
    DomainDecomposition,
    BulkSynchronous,
    DynamicTaskGraph,
    SecureAggregation,
    LoadBalanced,
    Speculative,
    Evolutionary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuRequirement {
    None,
    Optional,
    Required,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkClass {
    Low,
    Moderate,
    HighBandwidth,
    LowLatency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataVolume {
    Tiny,    // < 1 MB
    Small,   // 1 MB – 1 GB
    Medium,  // 1 – 100 GB
    Large,   // 100 GB – 10 TB
    Massive, // > 10 TB
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ResourceProfile {
    pub min_cores: u32,
    pub min_memory_mb: u64,
    pub gpu: GpuRequirement,
    pub network: NetworkClass,
    pub secure_enclave: bool,
    pub mobile_capable: bool,
}

/// A registered distributed computing workload type.
#[derive(Debug, Clone, Serialize)]
pub struct TaskProfile {
    pub id: &'static str,
    pub name: &'static str,
    pub category: TaskCategory,
    pub description: &'static str,
    pub scheduling: SchedulingStrategy,
    pub task_type: TaskType,
    pub resources: ResourceProfile,
    pub data_volume: DataVolume,
    pub urv_min: f64,
    pub urv_max: f64,
    pub tags: &'static [&'static str],
    pub bonsai_native: bool,
}

impl TaskProfile {
    /// Instantiate a runnable [`FabricTask`] from this profile.
    pub fn to_fabric_task(&self, project_id: impl Into<String>, payload: Vec<u8>, priority: u8) -> FabricTask {
        FabricTask {
            task_id: Uuid::new_v4().to_string(),
            project_id: project_id.into(),
            task_type: self.task_type.clone(),
            payload,
            priority,
            required_memory_mb: self.resources.min_memory_mb,
            required_cores: self.resources.min_cores,
        }
    }
}

// ─── Compact constructor ─────────────────────────────────────────────────────

const fn rp(cores: u32, mem_mb: u64, gpu: GpuRequirement, net: NetworkClass, enclave: bool, mobile: bool) -> ResourceProfile {
    ResourceProfile { min_cores: cores, min_memory_mb: mem_mb, gpu, network: net, secure_enclave: enclave, mobile_capable: mobile }
}

use GpuRequirement::*;
use NetworkClass::*;
use SchedulingStrategy::*;
use TaskCategory::*;
use TaskType::*;
use DataVolume::*;

// ─── The 50-task catalog ─────────────────────────────────────────────────────

pub const CATALOG: &[TaskProfile] = &[
    // ── AI & Machine Learning (8) ────────────────────────────────────────────
    TaskProfile {
        id: "federated-lora-aggregation",
        name: "Federated LoRA Aggregation",
        category: AiMl,
        description: "Devices train local LoRA adapters on private data; an aggregator merges them via FedAvg or TIES-merging to improve BonsAI without sharing raw data.",
        scheduling: ParameterServer,
        task_type: Inference,
        resources: rp(2, 2048, Optional, Moderate, false, true),
        data_volume: Small,
        urv_min: 0.05, urv_max: 0.50,
        tags: &["federated", "lora", "privacy", "training"],
        bonsai_native: true,
    },
    TaskProfile {
        id: "distributed-hyperparam-tuning",
        name: "Distributed Hyperparameter Tuning",
        category: AiMl,
        description: "Thousands of small training jobs across the grid of learning-rate / batch-size / LoRA-rank combinations; the aggregator selects the best configuration.",
        scheduling: EmbarrassinglyParallel,
        task_type: Inference,
        resources: rp(2, 4096, Optional, Low, false, false),
        data_volume: Medium,
        urv_min: 0.10, urv_max: 1.00,
        tags: &["hpo", "grid-search", "training"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "llm-inference-tensor-split",
        name: "LLM Inference Tensor Splitting",
        category: AiMl,
        description: "Run a 70B+ model across many devices by splitting layers; each holds a few layers and streams activations over the encrypted transfer lanes.",
        scheduling: PipelineParallel,
        task_type: Inference,
        resources: rp(4, 8192, Critical, HighBandwidth, false, false),
        data_volume: Medium,
        urv_min: 1.00, urv_max: 5.00,
        tags: &["llm", "tensor-parallel", "inference"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "distributed-hnsw-index",
        name: "Distributed HNSW Index Construction",
        category: AiMl,
        description: "Build a semantic index over millions of documents: each node embeds and indexes a shard, then a coordinator merges into a global HNSW graph.",
        scheduling: MapReduce,
        task_type: DataProcess,
        resources: rp(4, 8192, Optional, Moderate, false, false),
        data_volume: Large,
        urv_min: 0.50, urv_max: 2.00,
        tags: &["embeddings", "hnsw", "search", "kdb"],
        bonsai_native: true,
    },
    TaskProfile {
        id: "rl-self-play",
        name: "Reinforcement Learning Self-Play",
        category: AiMl,
        description: "Thousands of devices simulate game/RL environments to generate experience tuples; a central learner updates the policy A3C-style.",
        scheduling: ParameterServer,
        task_type: Inference,
        resources: rp(2, 2048, Optional, Low, false, true),
        data_volume: Small,
        urv_min: 0.10, urv_max: 1.00,
        tags: &["rl", "self-play", "a3c"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "multimodal-embedding-align",
        name: "Multimodal Embedding Alignment",
        category: AiMl,
        description: "Compute CLIP-style image+text embeddings over a large dataset and align them into a distributed FAISS/HNSW index.",
        scheduling: MapReduce,
        task_type: Inference,
        resources: rp(4, 8192, Required, Moderate, false, false),
        data_volume: Large,
        urv_min: 1.00, urv_max: 5.00,
        tags: &["clip", "multimodal", "embeddings"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "evolutionary-model-merge",
        name: "Evolutionary Model Merging",
        category: AiMl,
        description: "Hundreds of LoRA adapters are merged at varying ratios across devices; each merge is benchmarked and the fittest configurations breed the next generation.",
        scheduling: Evolutionary,
        task_type: Inference,
        resources: rp(4, 8192, Required, Moderate, false, false),
        data_volume: Medium,
        urv_min: 0.50, urv_max: 2.00,
        tags: &["model-merge", "evolutionary", "ties"],
        bonsai_native: true,
    },
    TaskProfile {
        id: "distributed-benchmark-eval",
        name: "Distributed BonsAI Benchmark Evaluation",
        category: AiMl,
        description: "A new model's 10,000+ prompt eval suite is sharded across devices; aggregated accuracy/latency determines whether the model is promoted.",
        scheduling: EmbarrassinglyParallel,
        task_type: Inference,
        resources: rp(2, 4096, Optional, Low, false, true),
        data_volume: Small,
        urv_min: 0.10, urv_max: 1.00,
        tags: &["eval", "benchmark", "promotion-gate"],
        bonsai_native: true,
    },

    // ── Software Build & CI/CD (4) ────────────────────────────────────────────
    TaskProfile {
        id: "distributed-cargo-compile",
        name: "Distributed Compilation",
        category: BuildCi,
        description: "Split a large Cargo/npm workspace into modules, compile on separate devices with sccache as the shared backend, then link.",
        scheduling: EmbarrassinglyParallel,
        task_type: Script,
        resources: rp(4, 4096, None, Moderate, false, false),
        data_volume: Medium,
        urv_min: 0.10, urv_max: 0.50,
        tags: &["build", "cargo", "sccache", "ci"],
        bonsai_native: true,
    },
    TaskProfile {
        id: "reproducible-build-verify",
        name: "Reproducible Build Verification",
        category: BuildCi,
        description: "Compile identical source on many devices and compare output hashes; any mismatch flags build impurity or hardware fault.",
        scheduling: EmbarrassinglyParallel,
        task_type: Script,
        resources: rp(2, 2048, None, Low, false, false),
        data_volume: Small,
        urv_min: 0.05, urv_max: 0.20,
        tags: &["reproducible", "verification", "supply-chain"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "dependency-mirror-cas",
        name: "Dependency Mirroring to CAS",
        category: BuildCi,
        description: "Download and verify all transitive dependencies (crates.io / npm) across devices, storing them content-addressed for offline reproducible builds.",
        scheduling: MapReduce,
        task_type: DataProcess,
        resources: rp(1, 1024, None, HighBandwidth, false, true),
        data_volume: Medium,
        urv_min: 0.02, urv_max: 0.10,
        tags: &["dependencies", "cas", "offline"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "ci-cache-prefetch",
        name: "CI Cache Prefetch",
        category: BuildCi,
        description: "Predict next build targets from commit history and pre-build them on idle devices, warming the shared cache before CI requests arrive.",
        scheduling: Speculative,
        task_type: Script,
        resources: rp(2, 4096, None, Moderate, false, false),
        data_volume: Medium,
        urv_min: 0.05, urv_max: 0.30,
        tags: &["ci", "cache", "speculative"],
        bonsai_native: true,
    },

    // ── Multimedia & Rendering (5) ────────────────────────────────────────────
    TaskProfile {
        id: "distributed-video-transcode",
        name: "Distributed Video Transcoding",
        category: Multimedia,
        description: "Split 4K/8K video into GOP segments, transcode each to target formats/resolutions on separate devices, then concatenate.",
        scheduling: EmbarrassinglyParallel,
        task_type: DataProcess,
        resources: rp(4, 4096, Optional, HighBandwidth, false, false),
        data_volume: Large,
        urv_min: 0.20, urv_max: 2.00,
        tags: &["video", "transcode", "ffmpeg"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "nerf-training",
        name: "Neural Radiance Field Training",
        category: Multimedia,
        description: "Distribute NeRF training: each device trains on a subset of camera views; an aggregator fuses them into a single radiance field.",
        scheduling: AllReduce,
        task_type: Inference,
        resources: rp(4, 8192, Required, HighBandwidth, false, false),
        data_volume: Medium,
        urv_min: 2.00, urv_max: 10.00,
        tags: &["nerf", "3d", "rendering"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "gaussian-splatting-recon",
        name: "3D Gaussian Splatting Reconstruction",
        category: Multimedia,
        description: "Large-scale 3D reconstruction from hundreds of images; each device computes Gaussians for a view subset and the results are merged with CRDT boundary sync.",
        scheduling: MapReduce,
        task_type: Inference,
        resources: rp(4, 8192, Required, HighBandwidth, false, false),
        data_volume: Medium,
        urv_min: 1.00, urv_max: 5.00,
        tags: &["gaussian-splatting", "3d", "reconstruction"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "ai-video-upscale",
        name: "AI Video Upscaling",
        category: Multimedia,
        description: "Upscale low-resolution video to 4K/8K with Real-ESRGAN; each device processes an independent clip in parallel.",
        scheduling: EmbarrassinglyParallel,
        task_type: Inference,
        resources: rp(2, 4096, Required, Moderate, false, false),
        data_volume: Large,
        urv_min: 1.00, urv_max: 5.00,
        tags: &["upscale", "esrgan", "video"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "blender-frame-render",
        name: "Distributed 3D Frame Rendering",
        category: Multimedia,
        description: "Render a high-quality 3D animation by splitting the frame range across devices; each renders an independent batch of frames.",
        scheduling: EmbarrassinglyParallel,
        task_type: Script,
        resources: rp(4, 8192, Required, Moderate, false, false),
        data_volume: Large,
        urv_min: 0.50, urv_max: 5.00,
        tags: &["blender", "render", "animation"],
        bonsai_native: false,
    },

    // ── Scientific & Engineering Simulation (6) ───────────────────────────────
    TaskProfile {
        id: "molecular-dynamics",
        name: "Molecular Dynamics Simulation",
        category: Simulation,
        description: "Simulate protein folding with spatial domain decomposition; each device computes forces for its region and exchanges halo data with neighbours.",
        scheduling: DomainDecomposition,
        task_type: Script,
        resources: rp(4, 4096, Optional, HighBandwidth, false, false),
        data_volume: Medium,
        urv_min: 0.50, urv_max: 2.00,
        tags: &["molecular-dynamics", "protein", "science"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "cfd-navier-stokes",
        name: "Computational Fluid Dynamics",
        category: Simulation,
        description: "Solve Navier-Stokes on a structured grid with block partitioning and boundary exchange between neighbouring blocks.",
        scheduling: DomainDecomposition,
        task_type: Script,
        resources: rp(8, 8192, None, HighBandwidth, false, false),
        data_volume: Medium,
        urv_min: 1.00, urv_max: 5.00,
        tags: &["cfd", "navier-stokes", "science"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "quantum-circuit-tensor-sim",
        name: "Quantum Circuit Simulation",
        category: Simulation,
        description: "Simulate a 40+ qubit circuit via tensor-network contraction; contraction sub-tasks are distributed as a dynamic task graph.",
        scheduling: DynamicTaskGraph,
        task_type: Script,
        resources: rp(8, 16384, None, HighBandwidth, false, false),
        data_volume: Medium,
        urv_min: 1.00, urv_max: 10.00,
        tags: &["quantum", "tensor-network", "simulation"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "graph-pagerank-bsp",
        name: "Large Graph PageRank",
        category: Simulation,
        description: "Compute PageRank on a billion-edge graph using a vertex-centric bulk-synchronous (Pregel) model.",
        scheduling: BulkSynchronous,
        task_type: DataProcess,
        resources: rp(4, 8192, None, HighBandwidth, false, false),
        data_volume: Medium,
        urv_min: 0.50, urv_max: 2.00,
        tags: &["graph", "pagerank", "pregel"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "monte-carlo-integration",
        name: "Monte Carlo Integration",
        category: Simulation,
        description: "High-dimensional Monte Carlo integration (e.g. high-energy physics cross-sections); each device computes an independent batch of samples.",
        scheduling: EmbarrassinglyParallel,
        task_type: Script,
        resources: rp(2, 2048, None, Low, false, true),
        data_volume: Small,
        urv_min: 0.02, urv_max: 0.10,
        tags: &["monte-carlo", "physics", "statistics"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "gnn-drug-discovery",
        name: "GNN Training for Drug Discovery",
        category: Simulation,
        description: "Train a graph neural network on a molecular dataset to predict protein-ligand binding affinity, sharing gradients via all-reduce.",
        scheduling: AllReduce,
        task_type: Inference,
        resources: rp(4, 16384, Required, HighBandwidth, false, false),
        data_volume: Large,
        urv_min: 1.00, urv_max: 5.00,
        tags: &["gnn", "drug-discovery", "science"],
        bonsai_native: false,
    },

    // ── Data Processing & Analytics (4) ───────────────────────────────────────
    TaskProfile {
        id: "distributed-sql-query",
        name: "Distributed SQL Query",
        category: DataAnalytics,
        description: "Run a complex SQL query over large CSV/Parquet datasets with distributed scan, shuffled join, and aggregation (Presto-style).",
        scheduling: MapReduce,
        task_type: DataProcess,
        resources: rp(4, 8192, None, HighBandwidth, false, false),
        data_volume: Large,
        urv_min: 0.20, urv_max: 1.00,
        tags: &["sql", "analytics", "olap"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "log-analysis-mapreduce",
        name: "Distributed Log Analysis",
        category: DataAnalytics,
        description: "Process terabytes of logs: parse, extract fields, and compute aggregations (error rates, percentiles) via map-reduce per file.",
        scheduling: MapReduce,
        task_type: DataProcess,
        resources: rp(2, 4096, None, HighBandwidth, false, false),
        data_volume: Large,
        urv_min: 0.10, urv_max: 0.50,
        tags: &["logs", "analytics", "observability"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "archive-dedup-blake3",
        name: "Archive Deduplication",
        category: DataAnalytics,
        description: "Compute BLAKE3 hashes for every file in a large archive across devices, then group to find and eliminate duplicates.",
        scheduling: MapReduce,
        task_type: DataProcess,
        resources: rp(2, 2048, None, Moderate, false, true),
        data_volume: Large,
        urv_min: 0.05, urv_max: 0.20,
        tags: &["dedup", "blake3", "storage"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "timeseries-forecast-ensemble",
        name: "Time-Series Forecasting Ensemble",
        category: DataAnalytics,
        description: "Train ARIMA / LSTM / Prophet models on different series segments across devices, then average their predictions.",
        scheduling: EmbarrassinglyParallel,
        task_type: Inference,
        resources: rp(2, 4096, Optional, Low, false, true),
        data_volume: Medium,
        urv_min: 0.10, urv_max: 0.50,
        tags: &["forecasting", "ensemble", "timeseries"],
        bonsai_native: false,
    },

    // ── Cryptography & Privacy (6) ────────────────────────────────────────────
    TaskProfile {
        id: "post-quantum-keygen-mpc",
        name: "Post-Quantum Key Generation (MPC)",
        category: Cryptography,
        description: "Generate ML-KEM (Kyber) / ML-DSA key pairs via multi-party computation; each device contributes randomness and no single node holds the full seed.",
        scheduling: SecureAggregation,
        task_type: Script,
        resources: rp(2, 2048, None, Moderate, true, true),
        data_volume: Tiny,
        urv_min: 0.05, urv_max: 0.20,
        tags: &["post-quantum", "mpc", "keygen"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "zk-snark-proof-gen",
        name: "Zero-Knowledge Proof Generation",
        category: Cryptography,
        description: "Parallelise zk-SNARK proof computation for a large circuit; each device computes a subset of the constraint system.",
        scheduling: MapReduce,
        task_type: Script,
        resources: rp(4, 8192, None, Moderate, false, false),
        data_volume: Small,
        urv_min: 0.10, urv_max: 1.00,
        tags: &["zk-snark", "zero-knowledge", "crypto"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "homomorphic-batch",
        name: "Homomorphic Encryption Batch Processing",
        category: Cryptography,
        description: "Run encrypted database queries over a partitioned dataset; results are combined without ever decrypting the data.",
        scheduling: MapReduce,
        task_type: DataProcess,
        resources: rp(4, 8192, None, Moderate, false, false),
        data_volume: Medium,
        urv_min: 0.10, urv_max: 0.50,
        tags: &["homomorphic", "fhe", "privacy"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "threshold-signature-frost",
        name: "Threshold Signature Signing (FROST)",
        category: Cryptography,
        description: "A quorum of devices jointly signs a message; each computes a share and the coordinator combines them — no device sees the full private key.",
        scheduling: SecureAggregation,
        task_type: Script,
        resources: rp(1, 512, None, Moderate, true, true),
        data_volume: Tiny,
        urv_min: 0.01, urv_max: 0.05,
        tags: &["threshold", "frost", "signature"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "secure-aggregation-dp",
        name: "Privacy-Preserving Aggregation",
        category: Cryptography,
        description: "Securely compute sum/average/histogram over private user data with distributed differential-privacy noise, with no trusted aggregator.",
        scheduling: SecureAggregation,
        task_type: DataProcess,
        resources: rp(1, 1024, None, Moderate, false, true),
        data_volume: Small,
        urv_min: 0.02, urv_max: 0.10,
        tags: &["differential-privacy", "mpc", "aggregation"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "zk-proof-of-execution",
        name: "Zero-Knowledge Proof of Correct Execution",
        category: Cryptography,
        description: "Each device proves via zk-SNARK that it executed a task correctly without revealing inputs, enabling trustless verification of fabric work.",
        scheduling: EmbarrassinglyParallel,
        task_type: Script,
        resources: rp(4, 8192, None, Moderate, false, false),
        data_volume: Small,
        urv_min: 0.10, urv_max: 1.00,
        tags: &["zk", "verifiable-compute", "trustless"],
        bonsai_native: true,
    },

    // ── Security & Program Analysis (4) ───────────────────────────────────────
    TaskProfile {
        id: "distributed-fuzzing-coverage",
        name: "Coverage-Guided Distributed Fuzzing",
        category: Security,
        description: "Fuzzers run across devices with different mutation strategies; coverage is aggregated swarm-wide to guide future mutations. Feeds the Forced Failure Finder.",
        scheduling: EmbarrassinglyParallel,
        task_type: Script,
        resources: rp(2, 4096, None, Moderate, false, false),
        data_volume: Medium,
        urv_min: 0.05, urv_max: 0.50,
        tags: &["fuzzing", "coverage", "f3", "security"],
        bonsai_native: true,
    },
    TaskProfile {
        id: "yara-malware-scan",
        name: "Signature-Based Malware Scanning",
        category: Security,
        description: "Scan a large file system against YARA rules by splitting the file list across devices.",
        scheduling: MapReduce,
        task_type: DataProcess,
        resources: rp(2, 2048, None, Moderate, false, true),
        data_volume: Large,
        urv_min: 0.02, urv_max: 0.10,
        tags: &["malware", "yara", "scanning"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "distributed-smt-solving",
        name: "Distributed SMT Solving",
        category: Security,
        description: "A large SMT formula from program verification is split by case-splitting; each device solves a sub-formula and the model is combined.",
        scheduling: DynamicTaskGraph,
        task_type: Script,
        resources: rp(4, 8192, None, Moderate, false, false),
        data_volume: Small,
        urv_min: 0.10, urv_max: 1.00,
        tags: &["smt", "z3", "verification"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "symbolic-execution",
        name: "Distributed Symbolic Execution",
        category: Security,
        description: "Symbolically execute a binary's functions across devices, generating path constraints; a verifier checks safety properties (no overflow, no UAF).",
        scheduling: DynamicTaskGraph,
        task_type: Script,
        resources: rp(4, 8192, None, Moderate, false, false),
        data_volume: Small,
        urv_min: 0.20, urv_max: 2.00,
        tags: &["symbolic-execution", "analysis", "security"],
        bonsai_native: false,
    },

    // ── Real-Time Edge & IoT (3) ──────────────────────────────────────────────
    TaskProfile {
        id: "collaborative-object-detection",
        name: "Collaborative Object Detection",
        category: EdgeIot,
        description: "Edge cameras stream low-res frames to nearby devices running lightweight detectors; a tracker merges detections across overlapping fields of view.",
        scheduling: LoadBalanced,
        task_type: Inference,
        resources: rp(2, 2048, Optional, LowLatency, false, true),
        data_volume: Small,
        urv_min: 0.05, urv_max: 0.20,
        tags: &["object-detection", "edge", "realtime"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "federated-anomaly-detection",
        name: "Federated Anomaly Detection",
        category: EdgeIot,
        description: "Industrial IoT sensors send encrypted feature vectors to nearby devices; each trains a local autoencoder and the federated model flags anomalies.",
        scheduling: ParameterServer,
        task_type: Inference,
        resources: rp(2, 2048, None, LowLatency, false, true),
        data_volume: Small,
        urv_min: 0.02, urv_max: 0.10,
        tags: &["anomaly", "iot", "federated"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "realtime-speech-transcription",
        name: "Real-Time Collaborative Transcription",
        category: EdgeIot,
        description: "Each participant's device transcribes its own speech with on-device Whisper; a coordinator merges transcripts. No audio leaves any device.",
        scheduling: LoadBalanced,
        task_type: Inference,
        resources: rp(2, 2048, Optional, LowLatency, false, true),
        data_volume: Small,
        urv_min: 0.02, urv_max: 0.10,
        tags: &["whisper", "transcription", "privacy"],
        bonsai_native: false,
    },

    // ── Bonsai Self-Improvement (8) — the differentiators ─────────────────────
    TaskProfile {
        id: "self-optimising-scheduler-evolution",
        name: "Self-Optimising Scheduler Evolution",
        category: BonsaiNative,
        description: "The fabric runs hundreds of scheduling-algorithm variants against historical task traces; a genetic loop breeds the best and hot-reloads it into the live CoordinatorActor.",
        scheduling: Evolutionary,
        task_type: Script,
        resources: rp(2, 2048, None, Low, false, true),
        data_volume: Small,
        urv_min: 0.05, urv_max: 0.20,
        tags: &["meta", "scheduler", "evolutionary", "self-improvement"],
        bonsai_native: true,
    },
    TaskProfile {
        id: "swarm-forced-failure-finder",
        name: "Swarm-Based Forced Failure Finder",
        category: BonsaiNative,
        description: "F³ fuzzing campaigns fan out across all idle devices, each testing different components (Tauri commands, WASM tools, CRDT merges). Crashes flow into the Survival KB.",
        scheduling: EmbarrassinglyParallel,
        task_type: Script,
        resources: rp(2, 4096, None, Low, false, false),
        data_volume: Medium,
        urv_min: 0.05, urv_max: 0.20,
        tags: &["f3", "fuzzing", "survival", "self-healing"],
        bonsai_native: true,
    },
    TaskProfile {
        id: "distributed-causal-fault-localisation",
        name: "Distributed Causal Fault Localisation",
        category: BonsaiNative,
        description: "Replay the Universe Event Log from the last good snapshot; each device tests a different causal hypothesis to pinpoint the event sequence that caused a failure.",
        scheduling: EmbarrassinglyParallel,
        task_type: Script,
        resources: rp(2, 2048, None, Low, false, true),
        data_volume: Small,
        urv_min: 0.05, urv_max: 0.20,
        tags: &["time-travel", "fault-localisation", "universe"],
        bonsai_native: true,
    },
    TaskProfile {
        id: "federated-survival-rule-synthesis",
        name: "Federated Survival Rule Synthesis",
        category: BonsaiNative,
        description: "Each device mines its local crash logs to propose new survival rules; an aggregator merges, deduplicates, and validates them against a shared corpus before deployment.",
        scheduling: SecureAggregation,
        task_type: Script,
        resources: rp(1, 1024, None, Low, false, true),
        data_volume: Small,
        urv_min: 0.01, urv_max: 0.05,
        tags: &["survival", "rule-synthesis", "self-healing"],
        bonsai_native: true,
    },
    TaskProfile {
        id: "agent-persona-evolution",
        name: "Agent Persona Evolution",
        category: BonsaiNative,
        description: "Each device mutates an agent's system prompt, temperature, and tool permissions; mutated agents are benchmarked and the fittest breed hyper-specialised personas.",
        scheduling: Evolutionary,
        task_type: Inference,
        resources: rp(2, 4096, Optional, Low, false, false),
        data_volume: Small,
        urv_min: 0.10, urv_max: 0.50,
        tags: &["agents", "persona", "evolutionary", "swarm"],
        bonsai_native: true,
    },
    TaskProfile {
        id: "distributed-semantic-index-kdb",
        name: "Distributed KDB Semantic Indexing",
        category: BonsaiNative,
        description: "All Bonsai docs, chat logs, and code are embedded and indexed: each device builds a local HNSW shard and a coordinator merges into the global Knowledge Database index.",
        scheduling: MapReduce,
        task_type: DataProcess,
        resources: rp(4, 8192, Optional, Moderate, false, false),
        data_volume: Large,
        urv_min: 0.50, urv_max: 2.00,
        tags: &["kdb", "semantic-search", "embeddings"],
        bonsai_native: true,
    },
    TaskProfile {
        id: "swarm-memory-consolidation",
        name: "Swarm-Wide Memory Consolidation",
        category: BonsaiNative,
        description: "Instead of a single DreamAgent, the swarm consolidates memory: each device processes a subset of the day's memory nodes and a central agent synthesises BONSAI.md.",
        scheduling: MapReduce,
        task_type: Inference,
        resources: rp(2, 2048, Optional, Low, false, true),
        data_volume: Small,
        urv_min: 0.02, urv_max: 0.10,
        tags: &["memory", "dreamagent", "consolidation"],
        bonsai_native: true,
    },
    TaskProfile {
        id: "distributed-axiom-proof-search",
        name: "Distributed Axiom Proof Search",
        category: BonsaiNative,
        description: "A conjecture is decomposed into lemmas; each device runs the Axiom tactic engine on a lemma and broadcasts proofs so others can unblock dependent goals.",
        scheduling: DynamicTaskGraph,
        task_type: Script,
        resources: rp(4, 8192, None, Moderate, false, false),
        data_volume: Small,
        urv_min: 0.10, urv_max: 1.00,
        tags: &["axiom", "proof-search", "formal-verification"],
        bonsai_native: true,
    },

    // ── Interoperability & Volunteer Computing (2) ────────────────────────────
    TaskProfile {
        id: "boinc-folding-bridge",
        name: "BOINC / Folding@home Bridge",
        category: Interop,
        description: "Run work units from established volunteer projects (Folding@home, Rosetta@home, SETI@home); credits earned convert to Bonsai credits.",
        scheduling: EmbarrassinglyParallel,
        task_type: Script,
        resources: rp(2, 2048, Optional, Moderate, false, true),
        data_volume: Medium,
        urv_min: 0.01, urv_max: 0.10,
        tags: &["boinc", "folding", "volunteer"],
        bonsai_native: false,
    },
    TaskProfile {
        id: "bonsai-genesis",
        name: "Bonsai-Genesis — The Swarm Evolves the Fabric",
        category: BonsaiNative,
        description: "The meta-task: each device proposes a modification to the fabric's scheduling, allocation, or fault-tolerance policy; the swarm evaluates it in a shadow environment and the best are rolled out live.",
        scheduling: Evolutionary,
        task_type: Script,
        resources: rp(2, 2048, None, Moderate, false, false),
        data_volume: Small,
        urv_min: 0.50, urv_max: 2.00,
        tags: &["meta", "self-evolving", "genesis", "self-improvement"],
        bonsai_native: true,
    },
];

// ─── Catalog query API ───────────────────────────────────────────────────────

pub fn get(id: &str) -> Option<&'static TaskProfile> {
    CATALOG.iter().find(|p| p.id == id)
}

pub fn by_category(category: TaskCategory) -> Vec<&'static TaskProfile> {
    CATALOG.iter().filter(|p| p.category == category).collect()
}

pub fn bonsai_native() -> Vec<&'static TaskProfile> {
    CATALOG.iter().filter(|p| p.bonsai_native).collect()
}

pub fn runnable_on(cores: u32, memory_mb: u64, has_gpu: bool) -> Vec<&'static TaskProfile> {
    CATALOG
        .iter()
        .filter(|p| {
            p.resources.min_cores <= cores
                && p.resources.min_memory_mb <= memory_mb
                && match p.resources.gpu {
                    GpuRequirement::Required | GpuRequirement::Critical => has_gpu,
                    _ => true,
                }
        })
        .collect()
}

pub fn mobile_friendly() -> Vec<&'static TaskProfile> {
    CATALOG.iter().filter(|p| p.resources.mobile_capable).collect()
}

pub fn affordable(max_urv_per_min: f64) -> Vec<&'static TaskProfile> {
    CATALOG.iter().filter(|p| p.urv_min <= max_urv_per_min).collect()
}

pub fn count() -> usize {
    CATALOG.len()
}

pub fn category_summary() -> Vec<(&'static str, usize)> {
    use TaskCategory::*;
    [AiMl, BuildCi, Multimedia, Simulation, DataAnalytics, Cryptography, Security, EdgeIot, BonsaiNative, Interop]
        .iter()
        .map(|c| (c.as_str(), by_category(*c).len()))
        .filter(|(_, n)| *n > 0)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_50_unique_tasks() {
        assert_eq!(count(), 50, "catalog must contain exactly 50 tasks");
        let mut ids: Vec<&str> = CATALOG.iter().map(|p| p.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 50, "all task ids must be unique");
    }

    #[test]
    fn urv_bands_are_well_formed() {
        for p in CATALOG {
            assert!(p.urv_min >= 0.0 && p.urv_min <= p.urv_max, "bad URV band for {}", p.id);
        }
    }

    #[test]
    fn lookup_and_instantiation_work() {
        let p = get("federated-lora-aggregation").expect("profile exists");
        let task = p.to_fabric_task("proj-1", vec![1, 2, 3], 5);
        assert_eq!(task.required_cores, p.resources.min_cores);
        assert_eq!(task.priority, 5);
        assert_eq!(task.project_id, "proj-1");
    }

    #[test]
    fn bonsai_native_tasks_present() {
        assert!(bonsai_native().len() >= 8, "expected at least 8 self-improvement tasks");
    }

    #[test]
    fn mobile_filter_excludes_gpu_critical() {
        for p in mobile_friendly() {
            assert_ne!(p.resources.gpu, GpuRequirement::Critical, "{} marked mobile but needs critical GPU", p.id);
        }
    }
}
