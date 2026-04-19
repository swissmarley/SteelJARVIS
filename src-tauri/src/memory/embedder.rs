use std::sync::{Mutex, OnceLock};

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

/// Wraps a `fastembed` text-embedding model behind a lazy, thread-safe
/// initializer. The model files are downloaded to the user's cache dir on
/// first use (~90MB for bge-small-en-v1.5). All embed() failures are soft —
/// callers fall back to keyword search when this returns Err.
pub struct Embedder {
    inner: OnceLock<Result<Mutex<TextEmbedding>, String>>,
}

impl Embedder {
    pub fn new() -> Self {
        Self {
            inner: OnceLock::new(),
        }
    }

    /// Returns the 384-dim embedding for `text` as a Vec<f32>.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        let model = self.inner.get_or_init(|| {
            TextEmbedding::try_new(
                InitOptions::new(EmbeddingModel::BGESmallENV15)
                    .with_show_download_progress(true),
            )
            .map(Mutex::new)
            .map_err(|e| format!("fastembed init failed: {e}"))
        });

        let mutex = model.as_ref().map_err(|e| e.clone())?;
        let mut model = mutex
            .lock()
            .map_err(|e| format!("fastembed mutex poisoned: {e}"))?;
        let mut vectors = model
            .embed(vec![text.to_string()], None)
            .map_err(|e| format!("fastembed embed failed: {e}"))?;
        vectors
            .pop()
            .ok_or_else(|| "fastembed returned empty result".to_string())
    }

    /// Cosine similarity between two embeddings of the same dimension.
    pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
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
}

impl Default for Embedder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identity_is_one() {
        let v = vec![0.1, 0.2, 0.3, 0.4];
        let sim = Embedder::cosine(&v, &v);
        assert!((sim - 1.0).abs() < 1e-5, "expected 1.0, got {sim}");
    }

    #[test]
    fn cosine_orthogonal_is_zero() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!(Embedder::cosine(&a, &b).abs() < 1e-5);
    }

    #[test]
    fn cosine_mismatched_len_returns_zero() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert_eq!(Embedder::cosine(&a, &b), 0.0);
    }

    // This test downloads the model on first run (~90MB) and is marked
    // `ignore` so `cargo test` stays fast by default. Run explicitly with:
    //   cargo test --manifest-path src-tauri/Cargo.toml -- --ignored embed
    #[test]
    #[ignore]
    fn similar_sentences_have_higher_similarity() {
        let e = Embedder::new();
        let coffee_a = e.embed("I really like coffee in the morning").unwrap();
        let coffee_b = e.embed("Espresso is my favorite morning drink").unwrap();
        let unrelated = e.embed("The capital of France is Paris").unwrap();
        let sim_similar = Embedder::cosine(&coffee_a, &coffee_b);
        let sim_unrelated = Embedder::cosine(&coffee_a, &unrelated);
        assert!(
            sim_similar > sim_unrelated,
            "similar pair sim {sim_similar} should exceed unrelated {sim_unrelated}"
        );
    }
}
