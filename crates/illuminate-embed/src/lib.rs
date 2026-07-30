//! illuminate-embed: sentence embeddings for illuminate.
//!
//! ## The `onnx` feature
//!
//! The real encoder runs `all-MiniLM-L6-v2` through `fastembed`, which pulls
//! `ort-sys` — and `ort-sys` **downloads an ONNX Runtime binary from a
//! third-party CDN at build time**. That breaks the build on any machine
//! without egress to it, which contradicts illuminate's local-first promise.
//!
//! So the ONNX path lives behind the `onnx` feature (**on by default** — the
//! shipped binary is unchanged). With `--no-default-features` the crate builds
//! with zero network access and [`EmbedEngine`] becomes a stub whose
//! constructors return [`EmbedError::Unavailable`].
//!
//! The stub keeps the *exact* signatures of the real engine, so every
//! downstream caller compiles untouched. Callers already handle
//! `EmbedEngine::new()` returning `Err` (model download can fail at runtime
//! too), which is what makes the degrade safe rather than a special case.

#[cfg(feature = "onnx")]
mod encoder;
#[cfg(feature = "onnx")]
pub use encoder::EmbedEngine;

#[cfg(not(feature = "onnx"))]
mod stub;
#[cfg(not(feature = "onnx"))]
pub use stub::EmbedEngine;

#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    #[error("model init failed: {0}")]
    ModelInit(String),
    #[error("encoding failed: {0}")]
    Encoding(String),
    /// Returned by every [`EmbedEngine`] constructor when the crate was built
    /// without the `onnx` feature. Callers should fall back to their
    /// non-semantic path (lexical search, deterministic ranking) rather than
    /// treating this as fatal.
    #[error("embeddings unavailable: built without the `onnx` feature")]
    Unavailable,
}

impl EmbedError {
    /// True when the failure is "this build has no embedding engine" rather
    /// than "the engine tried and failed". Lets callers distinguish a
    /// deliberately-slim build from a genuine runtime fault.
    pub fn is_unavailable(&self) -> bool {
        matches!(self, EmbedError::Unavailable)
    }
}
