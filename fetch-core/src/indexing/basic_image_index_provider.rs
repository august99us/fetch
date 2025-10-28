use crate::indexing::{store::{Embedded, KeyedData, KeyedSequencedStore}, ChunkFile};



pub struct BasicImageIndexProvider {
    // pub image_embedder: Siglip2ImageEmbedder;
    pub storage: Box<dyn KeyedSequencedStore<String, Siglip2EmbeddedChunkFile>>,
}

pub struct Siglip2EmbeddedChunkFile {
    pub chunkfile: ChunkFile,
    pub embedding: Vec<f32>,
}

impl Embedded<ChunkFile> for Siglip2EmbeddedChunkFile {
    fn take_data_and_vector(self) -> (ChunkFile, Vec<f32>) {
        (self.chunkfile, self.embedding)
    }
    
    fn embedding_length() -> u32 {
        768
    }
}

impl KeyedData<String> for Siglip2EmbeddedChunkFile {
    fn get_key(&self) -> String {
        self.chunkfile.get_key()
    }
}