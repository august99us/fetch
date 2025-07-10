use std::error::Error;

use crate::vector_store::{lancedb_store::LanceDBStore, IndexVector, QueryVectorKeys};

#[derive(thiserror::Error, Debug)]
pub enum FileIndexerError {
    #[error("Invalid dependency object provided to constructor")]
    InvalidDependency { dependency: &'static str, issue: &'static str },
    #[error("Dependency errored during construction")]
    DependencyError { dependency: &'static str, #[source] source: Box<dyn Error> },
}

#[derive(Clone)]
pub struct FileIndexer<I: IndexVector + QueryVectorKeys> {
    vector_store: I,
}
impl<I: IndexVector + QueryVectorKeys> FileIndexer<I> {
    // Testing constructor
    async fn new() -> Result<FileIndexer<LanceDBStore>, FileIndexerError> {
        let lancedbstore = LanceDBStore::new("./data_dir", 512).await
            .map_err(|e| FileIndexerError::DependencyError { dependency: "Lance Db Vector Store", 
                source: Box::new(e) })?;

        FileIndexer::with(lancedbstore)
    }
    pub fn with(vector_store: I) -> Result<FileIndexer<I>, FileIndexerError> {
        Ok(FileIndexer { vector_store })
    }
}

pub mod index_files;
pub mod query_files;