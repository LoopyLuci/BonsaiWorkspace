use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use rand::Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HnswError {
    #[error("index is empty")]
    Empty,
    #[error("dimension mismatch: expected {expected}, got {got}")]
    DimMismatch { expected: usize, got: usize },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, HnswError>;

// --- f16 bit manipulation (no external dep) ---

pub fn f32_to_f16(v: f32) -> u16 {
    let bits = v.to_bits();
    let sign = (bits >> 16) & 0x8000;
    let exp = ((bits >> 23) & 0xFF) as i32 - 127 + 15;
    let mantissa = (bits >> 13) & 0x3FF;
    if exp <= 0 {
        sign as u16
    } else if exp >= 31 {
        (sign as u16) | 0x7C00
    } else {
        (sign as u16) | ((exp as u16) << 10) | (mantissa as u16)
    }
}

pub fn f16_to_f32(v: u16) -> f32 {
    let sign = ((v & 0x8000) as u32) << 16;
    let exp = ((v >> 10) & 0x1F) as u32;
    let mantissa = (v & 0x3FF) as u32;
    let bits = if exp == 0 {
        sign | (mantissa << 13)
    } else if exp == 31 {
        sign | 0x7F800000 | (mantissa << 13)
    } else {
        sign | ((exp + 127 - 15) << 23) | (mantissa << 13)
    };
    f32::from_bits(bits)
}

// --- Distance metric ---

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Distance {
    Cosine,
    DotProduct,
    Euclidean,
}

impl Distance {
    pub fn compute(&self, a: &[f32], b: &[f32]) -> f32 {
        match self {
            Distance::Cosine => {
                let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
                let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
                let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
                if na == 0.0 || nb == 0.0 {
                    1.0
                } else {
                    1.0 - dot / (na * nb)
                }
            }
            Distance::DotProduct => {
                let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
                -dot
            }
            Distance::Euclidean => a
                .iter()
                .zip(b)
                .map(|(x, y)| (x - y).powi(2))
                .sum::<f32>()
                .sqrt(),
        }
    }
}

// --- Candidate for BinaryHeap (max-heap by distance) ---

#[derive(Debug, Clone, PartialEq)]
struct Candidate {
    dist: f32,
    id: usize,
}

impl Eq for Candidate {}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.dist
            .partial_cmp(&other.dist)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// --- Node ---

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Node {
    key_f16: Vec<u16>,
    layers: Vec<Vec<usize>>,
}

impl Node {
    fn new(key: &[f32], num_layers: usize) -> Self {
        Node {
            key_f16: key.iter().map(|&v| f32_to_f16(v)).collect(),
            layers: vec![Vec::new(); num_layers],
        }
    }

    fn key_f32(&self) -> Vec<f32> {
        self.key_f16.iter().map(|&v| f16_to_f32(v)).collect()
    }
}

// --- HNSW Index ---

#[derive(Debug, Serialize, Deserialize)]
pub struct HnswIndex {
    nodes: Vec<Node>,
    entry_point: Option<usize>,
    dim: usize,
    m: usize,      // max connections per layer
    m_max0: usize, // max connections on layer 0
    ef_construction: usize,
    ml: f64, // level multiplier = 1/ln(M)
    distance: Distance,
}

impl HnswIndex {
    pub fn new(dim: usize, m: usize, ef_construction: usize, distance: Distance) -> Self {
        HnswIndex {
            nodes: Vec::new(),
            entry_point: None,
            dim,
            m,
            m_max0: m * 2,
            ef_construction,
            ml: 1.0 / (m as f64).ln(),
            distance,
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    fn dist(&self, a: &[f32], b_id: usize) -> f32 {
        let b = self.nodes[b_id].key_f32();
        self.distance.compute(a, &b)
    }

    fn random_layer(&self) -> usize {
        let mut rng = rand::thread_rng();
        let r: f64 = rng.r#gen::<f64>();
        (-r.ln() * self.ml).floor() as usize
    }

    pub fn insert(&mut self, key: Vec<f32>) -> Result<usize> {
        if key.len() != self.dim {
            return Err(HnswError::DimMismatch {
                expected: self.dim,
                got: key.len(),
            });
        }

        let new_id = self.nodes.len();
        let level = self.random_layer();
        let node = Node::new(&key, level + 1);
        self.nodes.push(node);

        let Some(ep) = self.entry_point else {
            self.entry_point = Some(new_id);
            return Ok(new_id);
        };

        let max_level = self.nodes[ep].layers.len() - 1;
        let mut curr_ep = ep;

        // Greedy descent from top layer to level+1
        for lc in ((level + 1)..=max_level).rev() {
            let results = self.search_layer(&key, curr_ep, 1, lc);
            if let Some(best) = results
                .iter()
                .min_by(|a, b| a.dist.partial_cmp(&b.dist).unwrap())
            {
                curr_ep = best.id;
            }
        }

        // Insert and connect from level down to 0
        for lc in (0..=level.min(max_level)).rev() {
            let mut candidates = self.search_layer(&key, curr_ep, self.ef_construction, lc);
            candidates.sort_by(|a, b| a.dist.partial_cmp(&b.dist).unwrap());

            let m_layer = if lc == 0 { self.m_max0 } else { self.m };
            let neighbors: Vec<usize> = candidates.iter().take(m_layer).map(|c| c.id).collect();

            self.nodes[new_id].layers[lc] = neighbors.clone();

            for &nb in &neighbors {
                if !self.nodes[nb].layers[lc].contains(&new_id) {
                    self.nodes[nb].layers[lc].push(new_id);
                    if self.nodes[nb].layers[lc].len() > m_layer {
                        // Prune: keep closest — collect all data before mutating
                        let nb_key = self.nodes[nb].key_f32();
                        let layer_ids: Vec<usize> = self.nodes[nb].layers[lc].clone();
                        let mut scored: Vec<(f32, usize)> = layer_ids
                            .iter()
                            .map(|&id| {
                                (
                                    self.distance.compute(&nb_key, &self.nodes[id].key_f32()),
                                    id,
                                )
                            })
                            .collect();
                        scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                        self.nodes[nb].layers[lc] =
                            scored.into_iter().take(m_layer).map(|s| s.1).collect();
                    }
                }
            }

            if let Some(best) = candidates.first() {
                curr_ep = best.id;
            }
        }

        // Update entry point if new node has higher level
        if level > max_level {
            self.entry_point = Some(new_id);
        }

        Ok(new_id)
    }

    fn search_layer(&self, query: &[f32], ep: usize, ef: usize, level: usize) -> Vec<Candidate> {
        let ep_dist = self.dist(query, ep);
        let mut visited = std::collections::HashSet::new();
        visited.insert(ep);

        let mut candidates: BinaryHeap<std::cmp::Reverse<Candidate>> = BinaryHeap::new();
        candidates.push(std::cmp::Reverse(Candidate {
            dist: ep_dist,
            id: ep,
        }));

        let mut result: BinaryHeap<Candidate> = BinaryHeap::new();
        result.push(Candidate {
            dist: ep_dist,
            id: ep,
        });

        while let Some(std::cmp::Reverse(curr)) = candidates.pop() {
            let worst_result = result.peek().map(|c| c.dist).unwrap_or(f32::MAX);
            if curr.dist > worst_result && result.len() >= ef {
                break;
            }

            if level < self.nodes[curr.id].layers.len() {
                for &nb in &self.nodes[curr.id].layers[level].clone() {
                    if visited.insert(nb) {
                        let nb_dist = self.dist(query, nb);
                        let worst = result.peek().map(|c| c.dist).unwrap_or(f32::MAX);
                        if nb_dist < worst || result.len() < ef {
                            candidates.push(std::cmp::Reverse(Candidate {
                                dist: nb_dist,
                                id: nb,
                            }));
                            result.push(Candidate {
                                dist: nb_dist,
                                id: nb,
                            });
                            if result.len() > ef {
                                result.pop();
                            }
                        }
                    }
                }
            }
        }

        result.into_sorted_vec()
    }

    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<(usize, f32)>> {
        if query.len() != self.dim {
            return Err(HnswError::DimMismatch {
                expected: self.dim,
                got: query.len(),
            });
        }
        let Some(ep) = self.entry_point else {
            return Err(HnswError::Empty);
        };

        let max_level = self.nodes[ep].layers.len() - 1;
        let mut curr_ep = ep;

        for lc in (1..=max_level).rev() {
            let results = self.search_layer(query, curr_ep, 1, lc);
            if let Some(best) = results
                .iter()
                .min_by(|a, b| a.dist.partial_cmp(&b.dist).unwrap())
            {
                curr_ep = best.id;
            }
        }

        let ef = self.ef_construction.max(k);
        let mut candidates = self.search_layer(query, curr_ep, ef, 0);
        candidates.sort_by(|a, b| a.dist.partial_cmp(&b.dist).unwrap());

        Ok(candidates
            .into_iter()
            .take(k)
            .map(|c| (c.id, c.dist))
            .collect())
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let f = File::create(path)?;
        serde_json::to_writer(BufWriter::new(f), self)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let f = File::open(path)?;
        let idx = serde_json::from_reader(BufReader::new(f))?;
        Ok(idx)
    }

    /// Write all key vectors as raw f16 little-endian bytes (for .kmod keys.f16 file)
    pub fn save_keys_f16(&self, path: &Path) -> Result<()> {
        use std::io::Write;
        let mut f = BufWriter::new(File::create(path)?);
        for node in &self.nodes {
            for &v in &node.key_f16 {
                f.write_all(&v.to_le_bytes())?;
            }
        }
        Ok(())
    }
}

/// Convenience: build an index from a batch of vectors.
pub fn build(
    vectors: Vec<Vec<f32>>,
    m: usize,
    ef_construction: usize,
    distance: Distance,
) -> Result<HnswIndex> {
    let dim = vectors.first().map(|v| v.len()).unwrap_or(0);
    let mut idx = HnswIndex::new(dim, m, ef_construction, distance);
    for v in vectors {
        idx.insert(v)?;
    }
    Ok(idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn f16_roundtrip() {
        for &v in &[0.0f32, 1.0, -1.0, 0.5, 100.0, -0.001] {
            let encoded = f32_to_f16(v);
            let decoded = f16_to_f32(encoded);
            let err = (v - decoded).abs();
            assert!(
                err < 0.01 * v.abs().max(0.01),
                "f16 roundtrip failed for {v}: got {decoded}"
            );
        }
    }

    #[test]
    fn build_and_search() {
        let dim = 8;
        let mut vecs: Vec<Vec<f32>> = (0..20)
            .map(|i| (0..dim).map(|j| (i * dim + j) as f32 / 100.0).collect())
            .collect();

        let query = vecs[5].clone();
        let mut idx = HnswIndex::new(dim, 4, 16, Distance::Cosine);
        for v in vecs.drain(..) {
            idx.insert(v).unwrap();
        }

        let results = idx.search(&query, 3).unwrap();
        assert!(!results.is_empty());
        // Nearest neighbor should be the exact match (index 5)
        assert_eq!(results[0].0, 5);
    }

    #[test]
    fn save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.hnsw");

        let mut idx = HnswIndex::new(4, 4, 16, Distance::Euclidean);
        for i in 0..10u32 {
            idx.insert(vec![i as f32, 0.0, 0.0, 0.0]).unwrap();
        }

        idx.save(&path).unwrap();
        let loaded = HnswIndex::load(&path).unwrap();

        let results = loaded.search(&[5.0, 0.0, 0.0, 0.0], 1).unwrap();
        assert_eq!(results[0].0, 5);
    }

    #[test]
    fn dedup_neighbors() {
        let mut idx = HnswIndex::new(2, 4, 16, Distance::Euclidean);
        for i in 0..30 {
            idx.insert(vec![i as f32, 0.0]).unwrap();
        }
        // Ensure no node has duplicate neighbors
        for node in &idx.nodes {
            for layer in &node.layers {
                let unique: HashSet<usize> = layer.iter().cloned().collect();
                assert_eq!(unique.len(), layer.len(), "duplicate neighbors found");
            }
        }
    }
}
