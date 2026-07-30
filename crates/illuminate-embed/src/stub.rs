//! No-ONNX [`EmbedEngine`]: same surface as [`crate::encoder::EmbedEngine`],
//! but every constructor returns [`EmbedError::Unavailable`].
//!
//! Compiled only when the `onnx` feature is off. Nothing here links
//! `fastembed`/`ort-sys`, so this path builds with no network access.
//!
//! `cosine_similarity` is deliberately **still real** — it is pure arithmetic
//! over two slices with no model behind it, and callers that already hold
//! vectors (from `graph.db`, say) must keep working in a slim build.

use std::path::PathBuf;

use crate::EmbedError;

/// Stub engine. Cannot be constructed — [`EmbedEngine::new`] and
/// [`EmbedEngine::new_with_cache`] always fail — so the inference methods are
/// unreachable in practice. They exist to keep the type's surface identical to
/// the real encoder.
pub struct EmbedEngine {
    /// Uninhabited-in-practice marker: no field can be filled because no
    /// constructor succeeds.
    _private: (),
}

impl EmbedEngine {
    /// Always `Err(EmbedError::Unavailable)` in a build without `onnx`.
    pub fn new() -> Result<Self, EmbedError> {
        Err(EmbedError::Unavailable)
    }

    /// Always `Err(EmbedError::Unavailable)` in a build without `onnx`.
    pub fn new_with_cache(_cache_dir: PathBuf) -> Result<Self, EmbedError> {
        Err(EmbedError::Unavailable)
    }

    /// Unreachable — no instance can exist. Returns `Unavailable` for safety.
    pub fn embed(&self, _text: &str) -> Result<Vec<f32>, EmbedError> {
        Err(EmbedError::Unavailable)
    }

    /// Unreachable — no instance can exist. Returns `Unavailable` for safety.
    pub fn embed_batch(&self, _texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        Err(EmbedError::Unavailable)
    }

    /// Cosine similarity between two f32 vectors — identical to the real
    /// encoder's implementation. Pure; no model involved.
    /// Returns 0.0 if either vector has zero magnitude or the lengths differ.
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if mag_a == 0.0 || mag_b == 0.0 {
            0.0
        } else {
            dot / (mag_a * mag_b)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `expect_err` would require `EmbedEngine: Debug`, which the real encoder
    /// does not implement — deriving it only here would diverge the two
    /// surfaces. Match on the result instead.
    fn init_error(result: Result<EmbedEngine, EmbedError>) -> EmbedError {
        match result {
            Ok(_) => panic!("stub engine must not construct"),
            Err(e) => e,
        }
    }

    #[test]
    fn new_reports_unavailable_not_a_generic_failure() {
        let err = init_error(EmbedEngine::new());
        assert!(
            err.is_unavailable(),
            "callers distinguish a slim build from a runtime fault: {err}"
        );
    }

    #[test]
    fn new_with_cache_reports_unavailable() {
        let err = init_error(EmbedEngine::new_with_cache(PathBuf::from(
            "/tmp/does-not-matter",
        )));
        assert!(err.is_unavailable());
    }

    #[test]
    fn cosine_similarity_stays_real_in_a_slim_build() {
        let a = [1.0, 0.0, 0.0];
        let b = [1.0, 0.0, 0.0];
        assert!((EmbedEngine::cosine_similarity(&a, &b) - 1.0).abs() < f32::EPSILON);

        let c = [0.0, 1.0, 0.0];
        assert!(EmbedEngine::cosine_similarity(&a, &c).abs() < f32::EPSILON);
    }

    #[test]
    fn cosine_similarity_degenerate_inputs_are_zero() {
        assert_eq!(EmbedEngine::cosine_similarity(&[], &[]), 0.0);
        assert_eq!(EmbedEngine::cosine_similarity(&[1.0], &[1.0, 2.0]), 0.0);
        assert_eq!(
            EmbedEngine::cosine_similarity(&[0.0, 0.0], &[1.0, 1.0]),
            0.0
        );
    }
}
