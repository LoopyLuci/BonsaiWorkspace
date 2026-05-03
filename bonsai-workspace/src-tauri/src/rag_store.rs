/// Hybrid in-process RAG (Retrieval-Augmented Generation) store.
///
/// Combines two lexical signals and fuses them with Reciprocal Rank Fusion (RRF):
///   1. BM25 (stemmed query tokens, inverted index) — rewards term frequency + rarity.
///   2. Exact-match (un-stemmed lowercased terms) — rewards verbatim occurrences.
/// A path-relevance bonus is added when query terms appear in the file path.
///
/// Typical flow:
///   1. `index_directory(path, max_files)` — walk & chunk files at startup.
///   2. `search(query, top_k, path_filter)` — called by the search_knowledge tool.
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

// ── Scoring constants ──────────────────────────────────────────────────────────
const K1: f32 = 1.5;       // BM25 term-frequency saturation
const B:  f32 = 0.75;      // BM25 length normalisation
const RRF_K: f32 = 60.0;   // RRF rank smoothing constant (standard value)
const CHUNK_CHARS: usize = 512;
const CHUNK_OVERLAP: usize = 64;

// ── Data model ─────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct DocChunk {
    pub id:     usize,
    pub path:   String,
    pub text:   String,
    tokens:     Vec<String>,
}

// ── Global store ───────────────────────────────────────────────────────────────

struct RagStore {
    chunks:   RwLock<Vec<DocChunk>>,
    // inverted index: token → set of chunk ids
    inverted: RwLock<HashMap<String, Vec<usize>>>,
    // df (document frequency) per token
    df:       RwLock<HashMap<String, usize>>,
}

impl RagStore {
    fn new() -> Self {
        Self {
            chunks:   RwLock::new(Vec::new()),
            inverted: RwLock::new(HashMap::new()),
            df:       RwLock::new(HashMap::new()),
        }
    }

    fn is_empty(&self) -> bool {
        self.chunks.read().map(|c| c.is_empty()).unwrap_or(true)
    }

    fn add_chunk(&self, path: String, text: String) {
        let tokens = tokenize(&text);
        if tokens.is_empty() { return; }

        let id = {
            let mut chunks = self.chunks.write().unwrap();
            let id = chunks.len();
            chunks.push(DocChunk { id, path, text, tokens: tokens.clone() });
            id
        };

        // Update inverted index
        let unique_tokens: std::collections::HashSet<String> = tokens.iter().cloned().collect();
        {
            let mut inv = self.inverted.write().unwrap();
            for tok in &unique_tokens {
                inv.entry(tok.clone()).or_default().push(id);
            }
        }
        {
            let mut df = self.df.write().unwrap();
            for tok in &unique_tokens {
                *df.entry(tok.clone()).or_insert(0) += 1;
            }
        }
    }

    fn search(&self, query: &str, top_k: usize, path_filter: Option<&str>) -> Vec<(f32, DocChunk)> {
        let query_tokens = tokenize(query);
        if query_tokens.is_empty() { return Vec::new(); }

        // Exact (un-stemmed, lowercased) terms for the second signal.
        let exact_terms: Vec<String> = query.to_lowercase()
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|t| t.len() >= 2)
            .map(|t| t.to_string())
            .collect();

        let chunks_guard = self.chunks.read().unwrap();
        let df_guard     = self.df.read().unwrap();
        let n = chunks_guard.len() as f32;
        if n == 0.0 { return Vec::new(); }

        let avg_dl = chunks_guard.iter().map(|c| c.tokens.len() as f32).sum::<f32>() / n;

        // ── Signal 1: BM25 ──────────────────────────────────────────────────────
        let mut bm25: HashMap<usize, f32> = HashMap::new();
        for qt in &query_tokens {
            let df_t = *df_guard.get(qt).unwrap_or(&0) as f32;
            if df_t == 0.0 { continue; }
            let idf = ((n - df_t + 0.5) / (df_t + 0.5) + 1.0).ln();
            for chunk in chunks_guard.iter() {
                if let Some(pf) = path_filter { if !chunk.path.contains(pf) { continue; } }
                let tf = chunk.tokens.iter().filter(|t| *t == qt).count() as f32;
                if tf == 0.0 { continue; }
                let dl = chunk.tokens.len() as f32;
                let tf_norm = tf * (K1 + 1.0) / (tf + K1 * (1.0 - B + B * dl / avg_dl));
                *bm25.entry(chunk.id).or_insert(0.0) += idf * tf_norm;
            }
        }

        // ── Signal 2: exact-match ───────────────────────────────────────────────
        let mut exact: HashMap<usize, f32> = HashMap::new();
        if !exact_terms.is_empty() {
            for chunk in chunks_guard.iter() {
                if let Some(pf) = path_filter { if !chunk.path.contains(pf) { continue; } }
                let lowered = chunk.text.to_lowercase();
                let hits: usize = exact_terms.iter()
                    .map(|t| lowered.matches(t.as_str()).count())
                    .sum();
                if hits > 0 {
                    exact.insert(chunk.id, hits as f32);
                }
            }
        }

        // ── RRF fusion ──────────────────────────────────────────────────────────
        let mut bm25_sorted: Vec<(usize, f32)> = bm25.into_iter().collect();
        bm25_sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut exact_sorted: Vec<(usize, f32)> = exact.into_iter().collect();
        exact_sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut rrf: HashMap<usize, f32> = HashMap::new();
        for (rank, (id, _)) in bm25_sorted.iter().enumerate() {
            *rrf.entry(*id).or_insert(0.0) += 1.0 / (RRF_K + (rank + 1) as f32);
        }
        for (rank, (id, _)) in exact_sorted.iter().enumerate() {
            *rrf.entry(*id).or_insert(0.0) += 1.0 / (RRF_K + (rank + 1) as f32);
        }

        // ── Path-relevance bonus ────────────────────────────────────────────────
        // Boost chunks whose file path contains query terms (e.g. query "auth" → auth.rs).
        for chunk in chunks_guard.iter() {
            let path_lower = chunk.path.to_lowercase();
            let matches = exact_terms.iter().filter(|t| path_lower.contains(t.as_str())).count();
            if matches > 0 {
                // Scale: RRF scores are ~0.01-0.02; add ~0.01 per path match.
                *rrf.entry(chunk.id).or_insert(0.0) += matches as f32 * 0.01;
            }
        }

        let mut results: Vec<(f32, DocChunk)> = rrf
            .into_iter()
            .filter_map(|(id, score)| chunks_guard.get(id).map(|c| (score, c.clone())))
            .collect();

        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);
        results
    }
}

static STORE: OnceLock<RagStore> = OnceLock::new();

fn store() -> &'static RagStore {
    STORE.get_or_init(RagStore::new)
}

// ── Public API ──────────────────────────────────────────────────────────────────

pub fn global_rag() -> bool {
    // Returns true if the store has been populated
    !store().is_empty()
}

pub fn search(query: &str, top_k: usize, path_filter: Option<&str>) -> Vec<(f32, DocChunk)> {
    store().search(query, top_k, path_filter)
}

/// Index all text files under `root` up to `max_files` total.
pub fn index_directory(root: &str, max_files: usize) {
    let s = store();
    index_recursive(std::path::Path::new(root), s, &mut 0, max_files);
    eprintln!("[rag] indexed {} chunks from {root}", s.chunks.read().map(|c| c.len()).unwrap_or(0));
}

fn index_recursive(dir: &std::path::Path, s: &RagStore, count: &mut usize, max_files: usize) {
    if *count >= max_files { return; }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        if *count >= max_files { return; }
        let path = entry.path();
        // Skip hidden dirs and build artifacts
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') || matches!(name, "target" | "node_modules" | "dist" | ".git") {
                continue;
            }
        }
        if path.is_dir() {
            index_recursive(&path, s, count, max_files);
        } else if should_index(&path) {
            if let Ok(text) = std::fs::read_to_string(&path) {
                let path_str = path.display().to_string();
                for chunk in chunk_text(&text, CHUNK_CHARS, CHUNK_OVERLAP) {
                    s.add_chunk(path_str.clone(), chunk);
                }
                *count += 1;
            }
        }
    }
}

fn should_index(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).unwrap_or(""),
        "md" | "txt" | "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go"
        | "json" | "toml" | "yaml" | "yml" | "sh" | "html" | "css"
        | "svelte" | "vue" | "java" | "kt" | "swift" | "c" | "h" | "cpp"
    )
}

// ── Chunking ────────────────────────────────────────────────────────────────────

fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() { return Vec::new(); }

    let mut chunks = Vec::new();
    let mut start = 0usize;
    loop {
        let end = (start + chunk_size).min(chars.len());
        let chunk: String = chars[start..end].iter().collect();
        let trimmed = chunk.trim();
        if !trimmed.is_empty() {
            chunks.push(trimmed.to_string());
        }
        if end >= chars.len() { break; }
        start = end.saturating_sub(overlap);
    }
    chunks
}

// ── Tokenizer ──────────────────────────────────────────────────────────────────

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| t.len() >= 2 && t.len() <= 40)
        .map(|t| stem(t))
        .filter(|t| !is_stopword(t))
        .collect()
}

/// Minimal Porter-like stemmer (just the most common suffixes).
fn stem(word: &str) -> String {
    let w = word;
    if w.len() > 5 {
        for suffix in &["ings", "ing", "tion", "tions", "ed", "ly", "er", "est", "ies", "es", "s"] {
            if w.ends_with(suffix) && w.len() - suffix.len() >= 3 {
                return w[..w.len() - suffix.len()].to_string();
            }
        }
    }
    w.to_string()
}

fn is_stopword(w: &str) -> bool {
    matches!(w,
        "the" | "a" | "an" | "is" | "in" | "on" | "at" | "to" | "of"
        | "and" | "or" | "but" | "not" | "with" | "for" | "it" | "this"
        | "that" | "be" | "as" | "by" | "are" | "was" | "were" | "has"
        | "have" | "do" | "does" | "did" | "from" | "if" | "can" | "will"
        | "its" | "their" | "they" | "we" | "you" | "he" | "she" | "my"
        | "your" | "our" | "his" | "her" | "me" | "him" | "us" | "them"
    )
}
