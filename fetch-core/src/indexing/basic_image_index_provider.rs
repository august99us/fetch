use crate::indexing::{store::{lancedb::integrations::ArrowData, KeyedSequencedStore}, ChunkFile};

pub struct BasicImageIndexProvider {
    // pub image_embedder: Siglip2ImageEmbedder;
    pub vector_store: Box<dyn KeyedSequencedStore<String, Siglip2EmbeddedChunkFile>>,
}

pub struct Siglip2EmbeddedChunkFile {
    pub chunkfile: ChunkFile,
    pub embedding: Vec<f32>,
}

mod integrations;
pub use integrations::*;