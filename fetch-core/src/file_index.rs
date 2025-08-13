use std::error::Error;

use crate::vector_store::{lancedb_store::LanceDBStore, IndexVector, QueryVectorKeys};

/// Errors that can occur related to the file indexer object itself.
#[derive(thiserror::Error, Debug)]
pub enum FileIndexerError {
    /// A dependency failed during construction or initialization.
    /// 
    /// This error occurs when a required dependency (such as the vector store)
    /// encounters an error during its setup or initialization phase.
    #[error("Dependency errored during construction")]
    DependencyError { dependency: &'static str, #[source] source: Box<dyn Error> },
}

/// A file indexer that manages the indexing and querying of files semantically, 
/// by utilizing an embedding model and a vector store.
/// 
/// The `FileIndexer` is a generic struct that works with any vector store implementation
/// that supports both indexing vectors and querying for similar vectors by keys.
/// 
/// # Example
/// 
/// ```rust
/// use fetch::file_index::{FileIndexer, index_files::IndexFiles, query_files::QueryFiles};
/// use fetch::vector_store::lancedb_store::LanceDBStore;
/// use camino::Utf8Path;
/// 
/// async fn example() -> Result<(), Box<dyn std::error::Error>> {
///     // Create a new vector store and file indexer
///     let store = LanceDBStore::new("./data_dir", 512).await?;
///     let indexer = FileIndexer::with(store);
/// 
///     // Index a file
///     let path = Utf8Path::new("/path/to/image.jpg");
///     let result = indexer.index(path).await?;
/// 
///     // Query for semantically similar files
///     let results = indexer.query("a photo of a dog").await?;
///     let specific_results = indexer.query_n("sunset landscape", 10).await?;
///     Ok(())
/// }
/// ```
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

        Ok(FileIndexer::with(lancedbstore))
    }
    /// Creates a new `FileIndexer` with the provided vector store.
    /// 
    /// # Arguments
    /// 
    /// * `vector_store` - A vector store implementation that supports both indexing and querying
    /// 
    /// # Returns
    /// 
    /// A new `FileIndexer` instance configured with the given vector store.
    pub fn with(vector_store: I) -> FileIndexer<I> {
        FileIndexer { vector_store }
    }
}

pub mod index_files;
pub mod query_files;