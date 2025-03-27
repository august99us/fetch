use std::error::Error;

use embed_anything::embeddings::embed::{EmbedImage, Embedder};

use crate::{Preview, PreviewType};

/// Adds the embeddable trait, signifying that a struct or object has data that it can use to
/// create an embedding.
/// 
/// Using the Embedder trait here ties the API of fetch to the API of embed_anything. This is something
/// I am willing to commit to because the functions that embed_anything seems to intend to provide match
/// closely the functionalities that I am looking to satisfy with such a trait, were I to build one myself.
/// This was my previous intention with fetch-translation, however having found embed_anything I believe
/// this is no longer necessary.
/// 
/// Interestingly, the authors of embed_anything originally set out to do what looks like the same goal
/// as what I am trying to achieve with fetch, with their Starlight Search project.
/// https://starlight-search.com/blog/2024/12/15/embed-anything/
/// It sounds like their strategy was to locally embed the entire document, and therefore they ran into issues
/// with both large documents and locally embedding things. Solution for large documents was to stream the
/// document instead of loading the entire thing into memory, and for local embeddings they built embed_
/// anything. My strategy differs slightly in that I only intend to embed limited sized previews of files,
/// but I also don't yet have a solution for something like a pdf file (which is both on the larger side,
/// and also contains multiple modalities within the same file).
pub trait Embeddable {
    /// Calculates the embedding for the presented data in the objects using the Embedder passed in the
    /// arguments. Embedder model should support both image and text embeddings.
    async fn calculate_embedding(&self, embedder: &Embedder) -> Result<Vec<f32>, Box<dyn Error>>;
}

// Considering the API is already tied to embed_anything is this error still useful?
pub enum EmbeddingError {

}

impl<'a> Embeddable for Preview<'a> {
    async fn calculate_embedding(&self, embedder: &Embedder) -> Result<Vec<f32>, Box<dyn Error>> {
        match self.r#type {
            PreviewType::Image => {
                // TODO: make this implementation more mature, both using a better model and better code,
                // with error handling, etc.
                let result = embedder.embed_image(&self.path, None)?;
                Ok(result.embedding.to_dense().expect("expected dense vector returned from embedding model"))
            },
            _ => todo!(),
        }
    }
}

impl Embeddable for &str {
    async fn calculate_embedding(&self, embedder: &Embedder) -> Result<Vec<f32>, Box<dyn Error>> {
        let string_batch = [self.to_string()];
        let result = embedder.embed(&string_batch, Some(1)).await?;

        Ok(result.get(0).unwrap().to_dense().expect("expected dense vector returned from embedding model"))
    }
}