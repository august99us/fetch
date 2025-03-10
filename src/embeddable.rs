use std::{io::Read, sync::Arc};

use embed_anything::embeddings::embed::{EmbedImage, Embedder};

use crate::{Preview, PreviewType};

pub type Embedding = [f32; 512]; // change to parameterized? vector length

pub trait Embeddable {
    fn calculate_embedding(&self) -> Result<Embedding, String>;
}

impl<R: Read> Embeddable for Preview<R> {
    fn calculate_embedding(&self) -> Result<Embedding, String> {
        match self.r#type {
            PreviewType::Image => {
                // TODO: make this implementation more mature, both using a better model and better code, with error handling, etc.
                let embedder = Arc::new(Embedder::from_pretrained_hf("clip", "openai/clip-vit-base-patch32", None).unwrap());
                let result = embedder.embed_image(&self.path, None);
                result.map(|r| r.embedding.to_dense().unwrap().try_into().unwrap()).map_err(|e| e.to_string())
            },
            _ => todo!(),
        }
    }
}

impl Embeddable for &str {
    fn calculate_embedding(&self) -> Result<Embedding, String> {
        todo!()
    }
}