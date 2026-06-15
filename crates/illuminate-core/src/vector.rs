//! Vector similarity search behind a swappable [`VectorIndex`] trait.
//!
//! illuminate's semantic-search leg historically did a brute-force cosine scan
//! over every stored embedding — exact and deterministic, but O(N) per query.
//! This module puts that behind a trait with two implementations:
//!
//! - [`FlatIndex`] — the exact brute-force scan (the default; correct at any N,
//!   fully deterministic — what the cold-open CLI path uses).
//! - [`HnswIndex`] — a pure-Rust embedded HNSW (approximate nearest neighbor)
//!   for callers that build the index once and query it many times (a
//!   long-lived MCP server, batch search) and need to scale past the
//!   brute-force comfort zone.
//!
//! This is illuminate's answer to "should we adopt a graph+vector database?":
//! capture the one real win (an indexed ANN) **without** a server, a container,
//! or a closed engine — a pure-Rust crate that statically links and keeps the
//! single-binary / offline / one-file invariants intact. RRF fusion and the
//! FTS5 keyword leg are unchanged; only the nearest-neighbor leg is swappable.

use instant_distance::{Builder, HnswMap, Point, Search};

/// One ranked hit: `(id, cosine_similarity)` with similarity in `[-1, 1]`.
pub type Hit = (String, f32);

/// A nearest-neighbor index over `(id, embedding)` pairs.
pub trait VectorIndex {
    /// Top-`k` ids by cosine similarity, highest first.
    fn search(&self, query: &[f32], k: usize) -> Vec<Hit>;
    /// Number of indexed vectors.
    fn len(&self) -> usize;
    /// Whether the index is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Cosine similarity of two equal-length vectors. Returns 0.0 for a length
/// mismatch or a zero-norm vector.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

fn normalize(v: &[f32]) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        v.iter().map(|x| x / norm).collect()
    } else {
        v.to_vec()
    }
}

/// Exact brute-force index — the default. Deterministic and correct at any N;
/// O(N) per query. Building is free (it just holds the vectors).
pub struct FlatIndex {
    items: Vec<(String, Vec<f32>)>,
}

impl FlatIndex {
    /// Wrap `(id, embedding)` pairs for exact cosine search.
    pub fn new(items: Vec<(String, Vec<f32>)>) -> Self {
        Self { items }
    }
}

impl VectorIndex for FlatIndex {
    fn search(&self, query: &[f32], k: usize) -> Vec<Hit> {
        let mut scored: Vec<Hit> = self
            .items
            .iter()
            .map(|(id, v)| (id.clone(), cosine_similarity(query, v)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        scored
    }
    fn len(&self) -> usize {
        self.items.len()
    }
}

/// A point in the HNSW graph: a unit-normalized embedding (so Euclidean
/// distance is a monotone function of cosine similarity).
#[derive(Clone)]
struct EmbPoint(Vec<f32>);

impl Point for EmbPoint {
    fn distance(&self, other: &Self) -> f32 {
        // Euclidean distance over unit vectors. d² = 2(1 − cos), so smaller
        // distance ⇔ higher cosine similarity.
        let n = self.0.len().min(other.0.len());
        let mut sum = 0.0f32;
        for i in 0..n {
            let d = self.0[i] - other.0[i];
            sum += d * d;
        }
        sum.sqrt()
    }
}

/// Pure-Rust embedded approximate index (HNSW). Build once, query many. The
/// embedded HNSW means no server, no container, no closed engine — it links
/// straight into the single binary.
pub struct HnswIndex {
    map: HnswMap<EmbPoint, String>,
    len: usize,
}

impl HnswIndex {
    /// Build an HNSW over `(id, embedding)` pairs. O(N log N); amortize it
    /// across many queries.
    pub fn build(items: Vec<(String, Vec<f32>)>) -> Self {
        let len = items.len();
        let mut points = Vec::with_capacity(len);
        let mut values = Vec::with_capacity(len);
        for (id, v) in items {
            points.push(EmbPoint(normalize(&v)));
            values.push(id);
        }
        let map = Builder::default().build(points, values);
        Self { map, len }
    }
}

impl VectorIndex for HnswIndex {
    fn search(&self, query: &[f32], k: usize) -> Vec<Hit> {
        if self.len == 0 {
            return Vec::new();
        }
        let q = EmbPoint(normalize(query));
        let mut search = Search::default();
        self.map
            .search(&q, &mut search)
            .take(k)
            // d² = 2(1 − cos) ⇒ cos = 1 − d²/2
            .map(|item| (item.value.clone(), 1.0 - item.distance.powi(2) / 2.0))
            .collect()
    }
    fn len(&self) -> usize {
        self.len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<(String, Vec<f32>)> {
        vec![
            ("a".into(), vec![1.0, 0.0, 0.0]),
            ("b".into(), vec![0.9, 0.1, 0.0]),
            ("c".into(), vec![0.0, 1.0, 0.0]),
            ("d".into(), vec![0.0, 0.0, 1.0]),
        ]
    }

    #[test]
    fn cosine_basics() {
        assert!((cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
        assert_eq!(cosine_similarity(&[1.0], &[1.0, 2.0]), 0.0); // length mismatch
        assert_eq!(cosine_similarity(&[0.0, 0.0], &[1.0, 1.0]), 0.0); // zero norm
    }

    #[test]
    fn flat_index_ranks_by_cosine() {
        let idx = FlatIndex::new(items());
        let hits = idx.search(&[1.0, 0.0, 0.0], 2);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].0, "a"); // identical → top
        assert_eq!(hits[1].0, "b"); // closest neighbor
        assert!(hits[0].1 > hits[1].1);
    }

    #[test]
    fn flat_index_empty_and_len() {
        let idx = FlatIndex::new(vec![]);
        assert!(idx.is_empty());
        assert!(idx.search(&[1.0], 5).is_empty());
    }

    #[test]
    fn hnsw_finds_the_nearest_neighbor() {
        let idx = HnswIndex::build(items());
        assert_eq!(idx.len(), 4);
        let hits = idx.search(&[1.0, 0.05, 0.0], 2);
        assert!(!hits.is_empty());
        // The nearest unit vectors to [1,0.05,0] are a/b — the top hit is one of them.
        assert!(hits[0].0 == "a" || hits[0].0 == "b");
        // cosine reconstructed from euclidean is in range
        assert!(hits[0].1 > 0.9 && hits[0].1 <= 1.0001);
    }

    #[test]
    fn flat_and_hnsw_agree_on_top_hit() {
        let flat = FlatIndex::new(items());
        let hnsw = HnswIndex::build(items());
        let q = vec![0.0, 0.95, 0.05];
        assert_eq!(flat.search(&q, 1)[0].0, "c");
        assert_eq!(hnsw.search(&q, 1)[0].0, "c");
    }
}
