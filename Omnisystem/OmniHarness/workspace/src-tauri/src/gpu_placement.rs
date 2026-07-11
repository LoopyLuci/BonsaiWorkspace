//! GPU/CPU hybrid tensor placement.
//!
//! Replaces the previous all-or-nothing `--n-gpu-layers` cutoff for models
//! whose quantization was previously permanently blocklisted from any GPU
//! use (`model_orchestrator.rs::is_gpu_unsafe_quant`). Real on-disk GGUF
//! files confirm those models are *not* uniformly quantized — e.g.
//! `Bonsai-1.7B-IQ1_S.gguf` is 69.5% IQ1_S by weight, but norms/embedding/
//! output/attn_v/some ffn_down tensors are kept at F32/Q2_K/IQ2_XXS (a
//! deliberate llama.cpp quantization practice: sensitive tensors get higher
//! precision). The bundled `llama-server` binary supports
//! `-ot/--override-tensor <regex>=<buffer>` for per-tensor backend
//! placement and `-cmoe/-ncmoe` for keeping MoE expert weights on CPU —
//! this module decides how to use them.
//!
//! `GgmlType` here is the per-tensor type enum (from `tensor_info.ggml_type`
//! in `gguf.rs`) — a *different* enum from the whole-file
//! `general.file_type` (`llama_ftype`) decoded by `model_registry.rs::Quant`.
//! Conflating the two was a pre-existing bug (see `Quant::from_file_type`'s
//! doc comment); this module only ever reads real per-tensor data, never
//! the whole-file summary.

use crate::gguf::{GgufValue, TensorInfo};
use std::collections::HashMap;

// ── ggml_type ────────────────────────────────────────────────────────────────

/// Per-tensor `ggml_type` values, empirically confirmed against 6 real
/// on-disk GGUF files spanning 4 quant families (see `gguf.rs` tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
pub enum GgmlType {
    F32,
    F16,
    Q4_0,
    Q4_1,
    Q5_0,
    Q5_1,
    Q8_0,
    Q8_1,
    Q2_K,
    Q3_K,
    Q4_K,
    Q5_K,
    Q6_K,
    Q8_K,
    IQ2_XXS,
    IQ2_XS,
    IQ3_XXS,
    IQ1_S,
    IQ4_NL,
    IQ3_S,
    IQ2_S,
    IQ4_XS,
    I8,
    I16,
    I32,
    I64,
    F64,
    IQ1_M,
    BF16,
    TQ1_0,
    TQ2_0,
    Unknown(u32),
}

impl GgmlType {
    pub fn from_u32(n: u32) -> Self {
        match n {
            0 => Self::F32,
            1 => Self::F16,
            2 => Self::Q4_0,
            3 => Self::Q4_1,
            6 => Self::Q5_0,
            7 => Self::Q5_1,
            8 => Self::Q8_0,
            9 => Self::Q8_1,
            10 => Self::Q2_K,
            11 => Self::Q3_K,
            12 => Self::Q4_K,
            13 => Self::Q5_K,
            14 => Self::Q6_K,
            15 => Self::Q8_K,
            16 => Self::IQ2_XXS,
            17 => Self::IQ2_XS,
            18 => Self::IQ3_XXS,
            19 => Self::IQ1_S,
            20 => Self::IQ4_NL,
            21 => Self::IQ3_S,
            22 => Self::IQ2_S,
            23 => Self::IQ4_XS,
            24 => Self::I8,
            25 => Self::I16,
            26 => Self::I32,
            27 => Self::I64,
            28 => Self::F64,
            29 => Self::IQ1_M,
            30 => Self::BF16,
            34 => Self::TQ1_0,
            35 => Self::TQ2_0,
            n => Self::Unknown(n),
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::F32 => "F32".into(),
            Self::F16 => "F16".into(),
            Self::Q4_0 => "Q4_0".into(),
            Self::Q4_1 => "Q4_1".into(),
            Self::Q5_0 => "Q5_0".into(),
            Self::Q5_1 => "Q5_1".into(),
            Self::Q8_0 => "Q8_0".into(),
            Self::Q8_1 => "Q8_1".into(),
            Self::Q2_K => "Q2_K".into(),
            Self::Q3_K => "Q3_K".into(),
            Self::Q4_K => "Q4_K".into(),
            Self::Q5_K => "Q5_K".into(),
            Self::Q6_K => "Q6_K".into(),
            Self::Q8_K => "Q8_K".into(),
            Self::IQ2_XXS => "IQ2_XXS".into(),
            Self::IQ2_XS => "IQ2_XS".into(),
            Self::IQ3_XXS => "IQ3_XXS".into(),
            Self::IQ1_S => "IQ1_S".into(),
            Self::IQ4_NL => "IQ4_NL".into(),
            Self::IQ3_S => "IQ3_S".into(),
            Self::IQ2_S => "IQ2_S".into(),
            Self::IQ4_XS => "IQ4_XS".into(),
            Self::I8 => "I8".into(),
            Self::I16 => "I16".into(),
            Self::I32 => "I32".into(),
            Self::I64 => "I64".into(),
            Self::F64 => "F64".into(),
            Self::IQ1_M => "IQ1_M".into(),
            Self::BF16 => "BF16".into(),
            Self::TQ1_0 => "TQ1_0".into(),
            Self::TQ2_0 => "TQ2_0".into(),
            Self::Unknown(n) => format!("Unknown({n})"),
        }
    }

    /// Approximate bytes per element — used for VRAM-budget math, not for
    /// display (see `model_registry.rs::Quant::bits_per_weight` for the
    /// display-oriented whole-file equivalent).
    pub fn bytes_per_element(&self) -> f64 {
        match self {
            Self::F32 | Self::I32 => 4.0,
            Self::F16 | Self::BF16 | Self::I16 => 2.0,
            Self::F64 | Self::I64 => 8.0,
            Self::I8 => 1.0,
            Self::Q8_0 | Self::Q8_1 | Self::Q8_K => 1.06,
            Self::Q6_K => 0.82,
            Self::Q5_0 | Self::Q5_1 | Self::Q5_K => 0.70,
            Self::Q4_0 | Self::Q4_1 | Self::Q4_K | Self::IQ4_NL | Self::IQ4_XS => 0.57,
            Self::Q3_K | Self::IQ3_XXS | Self::IQ3_S => 0.45,
            Self::Q2_K | Self::IQ2_XXS | Self::IQ2_XS | Self::IQ2_S => 0.32,
            Self::TQ2_0 => 0.26,
            Self::IQ1_S | Self::IQ1_M => 0.20,
            Self::TQ1_0 => 0.21,
            Self::Unknown(_) => 1.0, // conservative overestimate
        }
    }

    /// Confirmed-crashing on this app's Vulkan GPU backend as soon as any
    /// layer containing them is offloaded — reproduced directly by running
    /// llama-server standalone (see the historical note preserved on
    /// `model_orchestrator.rs::is_gpu_unsafe_quant`). `TQ1_0`/`TQ2_0` are
    /// deliberately NOT included here — unverified either way, and the
    /// graduated retry ladder (`model_orchestrator.rs`) is the mechanism
    /// that learns per-model whether they're safe, rather than assuming.
    pub fn is_confirmed_gpu_unsafe(&self) -> bool {
        matches!(self, Self::IQ1_S | Self::IQ1_M)
    }
}

// ── MoE detection ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct MoeProfile {
    pub expert_count: u32,
    pub expert_used_count: u32,
    pub block_count: u32,
    /// The metadata key prefix this was detected under (e.g. `qwen35moe`) —
    /// GGUF MoE metadata keys are architecture-prefixed, not a fixed string.
    pub architecture_prefix: String,
}

/// Detects a Mixture-of-Experts architecture from GGUF metadata + tensor
/// names. Key names are `<architecture>.expert_count`, not a fixed
/// `llama.expert_count` — confirmed against a real file
/// (`qwen35moe.expert_count`). Cross-checked against tensor names (llama.cpp's
/// `_exps` suffix convention for stacked expert weights) so a stray
/// unrelated `.expert_count`-suffixed key can't false-positive.
pub fn detect_moe(metadata: &HashMap<String, GgufValue>, tensors: &[TensorInfo]) -> Option<MoeProfile> {
    let (key, val) = metadata
        .iter()
        .find(|(k, _)| k.ends_with(".expert_count"))?;
    let expert_count = val.as_u64().unwrap_or(0) as u32;
    if expert_count == 0 {
        return None;
    }
    if !tensors.iter().any(|t| t.name.contains("_exps")) {
        return None;
    }

    let arch_prefix = key
        .strip_suffix(".expert_count")
        .unwrap_or(key)
        .to_string();
    let expert_used_count = metadata
        .get(&format!("{arch_prefix}.expert_used_count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let block_count = metadata
        .iter()
        .find(|(k, _)| k.ends_with(".block_count"))
        .and_then(|(_, v)| v.as_u64())
        .unwrap_or(0) as u32;

    Some(MoeProfile {
        expert_count,
        expert_used_count,
        block_count,
        architecture_prefix: arch_prefix,
    })
}

// ── Tensor type classification ──────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct TensorTypeProfile {
    /// (type, tensor count, weighted element count), sorted by weighted
    /// element count descending.
    pub by_type: Vec<(GgmlType, usize, u64)>,
    pub total_elements: u64,
}

impl TensorTypeProfile {
    pub fn dominant_type(&self) -> Option<GgmlType> {
        self.by_type.first().map(|(t, _, _)| *t)
    }

    pub fn weight_fraction(&self, t: GgmlType) -> f64 {
        if self.total_elements == 0 {
            return 0.0;
        }
        self.by_type
            .iter()
            .find(|(ty, _, _)| *ty == t)
            .map(|(_, _, elems)| *elems as f64 / self.total_elements as f64)
            .unwrap_or(0.0)
    }

    /// Combined weight fraction of every confirmed-GPU-unsafe type present.
    pub fn unsafe_weight_fraction(&self) -> f64 {
        if self.total_elements == 0 {
            return 0.0;
        }
        let unsafe_elems: u64 = self
            .by_type
            .iter()
            .filter(|(t, _, _)| t.is_confirmed_gpu_unsafe())
            .map(|(_, _, e)| *e)
            .sum();
        unsafe_elems as f64 / self.total_elements as f64
    }
}

pub fn classify_tensor_types(tensors: &[TensorInfo]) -> TensorTypeProfile {
    let mut counts: HashMap<GgmlType, (usize, u64)> = HashMap::new();
    let mut total_elements = 0u64;
    for t in tensors {
        let ty = GgmlType::from_u32(t.ggml_type);
        let elems = t.element_count();
        total_elements += elems;
        let entry = counts.entry(ty).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += elems;
    }
    let mut by_type: Vec<(GgmlType, usize, u64)> =
        counts.into_iter().map(|(t, (c, e))| (t, c, e)).collect();
    by_type.sort_by(|a, b| b.2.cmp(&a.2));
    TensorTypeProfile {
        by_type,
        total_elements,
    }
}

// ── Placement plan ───────────────────────────────────────────────────────────

/// A file is treated as "uniformly unsafe" (no partial-offload benefit
/// possible) once confirmed-unsafe types cover this much of the model by
/// weight — below this, pinning just the unsafe tensors to CPU via `-ot`
/// while offloading the rest is worth attempting.
const UNIFORM_UNSAFE_THRESHOLD: f64 = 0.97;

/// VRAM headroom reserved for KV cache + compute buffers + fragmentation
/// margin — mirrors `model_orchestrator.rs::estimate_safe_gpu_layers`'s
/// existing constant.
const VRAM_HEADROOM_MB: u64 = 2048;

#[derive(Debug, Clone, PartialEq)]
pub enum PlacementPlan {
    /// No tensor-safety or MoE concerns — offload the given flat layer
    /// count exactly as the pre-existing `--n-gpu-layers` heuristic decides.
    FullGpu(u32),
    /// Confirmed-unsafe tensor-name patterns must stay on CPU; everything
    /// else offloads normally via `--n-gpu-layers`.
    TensorOverride { ot_rules: Vec<String>, gpu_layers: u32 },
    /// MoE expert weights kept on CPU (first N layers); dense/shared layers
    /// still offload normally via `gpu_layers`.
    CpuMoe { n_cpu_moe: u32, gpu_layers: u32 },
    /// Both axes apply independently to the same model.
    Combined {
        ot_rules: Vec<String>,
        n_cpu_moe: u32,
        gpu_layers: u32,
    },
    /// No safe partial-GPU placement found — matches the previous
    /// behavior for genuinely uniformly-unsafe files (e.g. TQ1_0/TQ2_0, if
    /// confirmed unsafe and uniform).
    CpuOnly,
}

/// Strips a `blk.<N>.` prefix to get the block-generic tensor "role" (e.g.
/// `blk.3.ffn_gate.weight` → `ffn_gate.weight`) — per-role quantization is
/// uniform across blocks in practice, so one `-ot` rule per role (rather
/// than one per exact tensor name) covers every block compactly.
fn tensor_role(name: &str) -> &str {
    if let Some(rest) = name.strip_prefix("blk.") {
        if let Some(dot_idx) = rest.find('.') {
            if rest[..dot_idx].chars().all(|c| c.is_ascii_digit()) {
                return &rest[dot_idx + 1..];
            }
        }
    }
    name
}

/// GGUF tensor names only ever contain `[A-Za-z0-9_.]` in practice; `.` is
/// the only regex metacharacter among them, so a targeted escape (rather
/// than a full regex-metachar escaper) is sufficient here.
fn escape_dots(s: &str) -> String {
    s.replace('.', "\\.")
}

/// Builds compact `-ot`/`--override-tensor` regex rules (each forced to
/// `"CPU"`) from a list of individually-unsafe tensor names — one rule per
/// distinct block-generic role (e.g. one rule covers `ffn_gate` in every
/// block) rather than one rule per exact tensor name.
fn build_override_tensor_rules(unsafe_tensor_names: &[String]) -> Vec<String> {
    let mut roles: Vec<&str> = unsafe_tensor_names
        .iter()
        .map(|n| tensor_role(n))
        .collect();
    roles.sort_unstable();
    roles.dedup();
    roles
        .iter()
        .map(|role| format!("blk\\.\\d+\\.{}$=CPU", escape_dots(role)))
        .collect()
}

/// Estimates how many leading MoE layers' expert weights should stay on CPU
/// (`--n-cpu-moe N`) given available VRAM — conservative by design (biases
/// toward more CPU-MoE layers than strictly necessary); the graduated retry
/// ladder in `model_orchestrator.rs` self-corrects from real load outcomes.
fn estimate_cpu_moe_layers(tensors: &[TensorInfo], moe: &MoeProfile, vram_budget_mb: u64) -> u32 {
    if moe.block_count == 0 {
        return 0;
    }
    let expert_tensors: Vec<&TensorInfo> = tensors
        .iter()
        .filter(|t| t.name.contains("_exps"))
        .collect();
    if expert_tensors.is_empty() {
        return moe.block_count; // can't estimate — conservative: keep it all on CPU
    }

    let total_expert_elements: u64 = expert_tensors.iter().map(|t| t.element_count()).sum();
    // Use the dominant expert-tensor type's byte size for the estimate.
    let dominant_type = classify_tensor_types(&expert_tensors.iter().map(|&t| t.clone()).collect::<Vec<_>>())
        .dominant_type()
        .unwrap_or(GgmlType::Q4_K);
    let per_layer_elements = total_expert_elements / moe.block_count as u64;
    let per_layer_mb =
        (per_layer_elements as f64 * dominant_type.bytes_per_element() / (1024.0 * 1024.0)).ceil() as u64;
    if per_layer_mb == 0 {
        return 0;
    }

    let usable_mb = vram_budget_mb.saturating_sub(VRAM_HEADROOM_MB);
    let gpu_capable_layers = (usable_mb / per_layer_mb).min(moe.block_count as u64) as u32;
    moe.block_count.saturating_sub(gpu_capable_layers)
}

/// Builds a GPU/CPU placement plan for a model from its real tensor data —
/// the core decision this whole module exists for. Two independent axes:
/// per-tensor quant safety (`-ot`) and MoE expert placement (`-ncmoe`), both
/// of which can apply to the same model.
pub fn build_placement_plan(
    tensors: &[TensorInfo],
    moe: Option<&MoeProfile>,
    vram_budget_mb: u64,
    preferred_gpu_layers: u32,
) -> PlacementPlan {
    let profile = classify_tensor_types(tensors);
    let unsafe_fraction = profile.unsafe_weight_fraction();

    let ot_rules = if unsafe_fraction > 0.0 && unsafe_fraction < UNIFORM_UNSAFE_THRESHOLD {
        let unsafe_names: Vec<String> = tensors
            .iter()
            .filter(|t| GgmlType::from_u32(t.ggml_type).is_confirmed_gpu_unsafe())
            .map(|t| t.name.clone())
            .collect();
        Some(build_override_tensor_rules(&unsafe_names))
    } else {
        None
    };

    let n_cpu_moe = moe.map(|m| estimate_cpu_moe_layers(tensors, m, vram_budget_mb));

    match (ot_rules, n_cpu_moe) {
        (Some(ot), Some(ncmoe)) => PlacementPlan::Combined {
            ot_rules: ot,
            n_cpu_moe: ncmoe,
            gpu_layers: preferred_gpu_layers,
        },
        (Some(ot), None) => PlacementPlan::TensorOverride {
            ot_rules: ot,
            gpu_layers: preferred_gpu_layers,
        },
        (None, Some(ncmoe)) => PlacementPlan::CpuMoe {
            n_cpu_moe: ncmoe,
            gpu_layers: preferred_gpu_layers,
        },
        (None, None) => {
            if unsafe_fraction >= UNIFORM_UNSAFE_THRESHOLD {
                // Confirmed-unsafe type covers the whole model — no tensor
                // is left to offload. Honest, documented limit: this is
                // the TQ1_0/TQ2_0-shaped case, not a bug to paper over.
                PlacementPlan::CpuOnly
            } else {
                PlacementPlan::FullGpu(preferred_gpu_layers)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn tensor(name: &str, elements: u64, ggml_type: u32) -> TensorInfo {
        TensorInfo {
            name: name.to_string(),
            dims: vec![elements],
            ggml_type,
            offset: 0,
        }
    }

    #[test]
    fn classify_tensor_types_computes_weighted_fractions() {
        let tensors = vec![
            tensor("blk.0.ffn_gate.weight", 700, 19), // IQ1_S
            tensor("blk.0.attn_norm.weight", 300, 0), // F32
        ];
        let profile = classify_tensor_types(&tensors);
        assert_eq!(profile.total_elements, 1000);
        assert!((profile.weight_fraction(GgmlType::IQ1_S) - 0.7).abs() < 1e-9);
        assert!((profile.unsafe_weight_fraction() - 0.7).abs() < 1e-9);
    }

    #[test]
    fn build_placement_plan_uses_tensor_override_for_mixed_precision_model() {
        let tensors = vec![
            tensor("blk.0.ffn_gate.weight", 700, 19), // IQ1_S — unsafe
            tensor("blk.0.attn_norm.weight", 300, 0), // F32 — safe
        ];
        let plan = build_placement_plan(&tensors, None, 8192, 20);
        match plan {
            PlacementPlan::TensorOverride { ot_rules, gpu_layers } => {
                assert_eq!(gpu_layers, 20);
                assert_eq!(ot_rules.len(), 1);
                assert!(ot_rules[0].contains("ffn_gate"));
                assert!(ot_rules[0].ends_with("=CPU"));
            }
            other => panic!("expected TensorOverride, got {other:?}"),
        }
    }

    #[test]
    fn build_placement_plan_falls_back_to_cpu_only_for_uniform_unsafe_model() {
        let tensors = vec![
            tensor("blk.0.ffn_gate.weight", 999, 19), // IQ1_S
            tensor("blk.0.attn_norm.weight", 1, 0),   // negligible F32
        ];
        let plan = build_placement_plan(&tensors, None, 8192, 20);
        assert_eq!(plan, PlacementPlan::CpuOnly);
    }

    #[test]
    fn build_placement_plan_full_gpu_when_no_unsafe_tensors() {
        let tensors = vec![tensor("blk.0.attn_q.weight", 1000, 12)]; // Q4_K
        let plan = build_placement_plan(&tensors, None, 8192, 20);
        assert_eq!(plan, PlacementPlan::FullGpu(20));
    }

    #[test]
    fn detect_moe_requires_both_metadata_key_and_expert_tensor() {
        let mut metadata = HashMap::new();
        metadata.insert("qwen35moe.expert_count".to_string(), GgufValue::U32(256));
        metadata.insert(
            "qwen35moe.expert_used_count".to_string(),
            GgufValue::U32(8),
        );
        metadata.insert("qwen35moe.block_count".to_string(), GgufValue::U32(48));

        // No expert tensor present — must not detect MoE from metadata alone.
        let no_expert_tensors = vec![tensor("blk.0.attn_q.weight", 1000, 12)];
        assert!(detect_moe(&metadata, &no_expert_tensors).is_none());

        let with_expert_tensors = vec![tensor("blk.0.ffn_gate_exps.weight", 1000, 12)];
        let profile = detect_moe(&metadata, &with_expert_tensors).unwrap();
        assert_eq!(profile.expert_count, 256);
        assert_eq!(profile.expert_used_count, 8);
        assert_eq!(profile.block_count, 48);
        assert_eq!(profile.architecture_prefix, "qwen35moe");
    }

    #[test]
    fn build_placement_plan_uses_cpu_moe_for_moe_model() {
        let moe = MoeProfile {
            expert_count: 256,
            expert_used_count: 8,
            block_count: 4,
            architecture_prefix: "qwen35moe".to_string(),
        };
        // 4 layers of expert tensors, each ~543 MB (1B elements * 0.57
        // bytes/elem for Q4_K) — large enough that only some fit in a
        // constrained VRAM budget.
        let tensors: Vec<TensorInfo> = (0..4)
            .map(|i| tensor(&format!("blk.{i}.ffn_gate_exps.weight"), 1_000_000_000, 12))
            .collect();
        let plan = build_placement_plan(&tensors, Some(&moe), 4096, 20);
        match plan {
            PlacementPlan::CpuMoe { n_cpu_moe, .. } => assert!(n_cpu_moe > 0),
            other => panic!("expected CpuMoe, got {other:?}"),
        }
    }

    /// Cross-checks MoE detection + placement against the real Qwen3.6 MoE
    /// model on disk. Machine-specific fixture, gated behind `#[ignore]`.
    #[test]
    #[ignore]
    fn real_moe_model_is_detected_and_planned() {
        let path = Path::new(
            r"D:\Models\general\Qwen3.6-35B-A3B-Claude-4.7-Opus-Reasoning-Distilled-APEX-I-Compact.gguf\Qwen3.6-35B-A3B-Claude-4.7-Opus-Reasoning-Distilled-APEX-I-Compact.gguf",
        );
        if !path.exists() {
            return;
        }
        let (metadata, tensors) = crate::gguf::parse_gguf_tensor_info(path).unwrap();
        let moe = detect_moe(&metadata, &tensors);
        assert!(moe.is_some(), "expected MoE detection on a known MoE model");
        let moe = moe.unwrap();
        assert!(moe.expert_count > 1);

        let plan = build_placement_plan(&tensors, Some(&moe), 24_000, 40);
        assert!(matches!(
            plan,
            PlacementPlan::CpuMoe { .. } | PlacementPlan::Combined { .. }
        ));
    }
}
