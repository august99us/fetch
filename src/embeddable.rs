use std::{error::Error, io::Read, sync::Arc};

use embed_anything::embeddings::{embed::{EmbedImage, Embedder}, local::clip::ClipEmbedder};

use crate::{Preview, PreviewType};

pub type Embedding = [f32; 512]; // change to parameterized? vector length

pub trait Embeddable {
    fn calculate_embedding(&self) -> Result<Embedding, Box<dyn Error>>;
}

impl<R: Read> Embeddable for Preview<R> {
    fn calculate_embedding(&self) -> Result<Embedding, Box<dyn Error>> {
        match self.r#type {
            PreviewType::Image => {
                // TODO: make this implementation more mature, both using a better model and better code, with error handling, etc.
                let embedder = Arc::new(Embedder::from_pretrained_hf("clip", "openai/clip-vit-base-patch32", None).unwrap());
                let result = embedder.embed_image(&self.path, None);
                Ok(result?.embedding.to_dense().unwrap().try_into().unwrap())
            },
            _ => todo!(),
        }
    }
}

impl Embeddable for &str {
    fn calculate_embedding(&self) -> Result<Embedding, Box<dyn Error>> {
        let embedder = Arc::new(ClipEmbedder::new("openai/clip-vit-base-patch32".to_string(), None).unwrap());
        let string_batch = [self.to_string()];
        let result = embedder.embed(&string_batch, Some(1))?;

        Ok(result.get(0).unwrap().to_dense().unwrap().try_into().unwrap())
    }
}