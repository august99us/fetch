use std::error::Error;

use embed_anything::embeddings::embed::Embedder;

use crate::vector_store::{lancedb_store::LanceDBStore, IndexVector, QueryVectorKeys};

#[derive(thiserror::Error, Debug)]
pub enum FileIndexerError {
    #[error("Invalid dependency object provided to constructor")]
    InvalidDependency { dependency: &'static str, issue: &'static str },
    #[error("Dependency errored during construction")]
    DependencyError { dependency: &'static str, #[source] source: Box<dyn Error> },
}

pub struct FileIndexer<I: IndexVector + QueryVectorKeys> {
    embedder: Embedder,
    vector_store: I,
}
impl<I: IndexVector + QueryVectorKeys> FileIndexer<I> {
    pub async fn new() -> Result<FileIndexer<impl IndexVector + QueryVectorKeys>, FileIndexerError> {
        let embedder = Embedder::from_pretrained_hf("clip", "openai/clip-vit-base-patch32", None).unwrap();
        let lancedbstore = LanceDBStore::new("./data_dir", 512).await.map_err(|e| 
            FileIndexerError::DependencyError { dependency: "Lance Db Vector Store", source: Box::new(e) })?;

        FileIndexer::with(embedder, lancedbstore)
    }
    pub fn with(embedder: Embedder, vector_store: I) -> Result<FileIndexer<impl IndexVector + QueryVectorKeys>, FileIndexerError> {
        if let Embedder::Text(_) = embedder {
            return Err(FileIndexerError::InvalidDependency { dependency: "Embedder", 
                issue: "Was text embedder" });
        }

        Ok(FileIndexer { embedder, vector_store })
    }
}

pub mod index_files;
pub mod query_files;