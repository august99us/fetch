use std::{error::Error, sync::Arc};

use crate::{file_index::pagination::QueryCursor, index::{ChunkingIndexProvider, basic_image_index_provider::BasicImageIndexProvider}, store::{ClearByFilter, KeyedSequencedStore, lancedb::LanceDBStore}};

/// Errors that can occur related to the file indexer object itself.
#[derive(thiserror::Error, Debug)]
pub enum FileIndexError {
    /// A dependency failed during construction or initialization.
    /// 
    /// This error occurs when a required dependency (such as the vector store)
    /// encounters an error during its setup or initialization phase.
    #[error("Dependency errored during construction")]
    DependencyError { dependency: &'static str, #[source] source: Box<dyn Error> },
}

#[derive(Clone)]
pub struct FileIndexer
{
    index_providers: Vec<Arc<dyn ChunkingIndexProvider>>,
}

impl FileIndexer
{
    // Testing constructor
    async fn new() -> Result<FileIndexer, FileIndexError> {
        let basic_image = BasicImageIndexProvider::using(
            LanceDBStore::local("./data_dir", "basic_image_index".to_owned()).await
            .map_err(|e| FileIndexError::DependencyError {
                dependency: "Lance Db Vector Store", 
                source: Box::new(e)
            })?,
        );

        Ok(FileIndexer::with(vec![Arc::new(basic_image)]))
    }

    pub fn with(providers: Vec<Arc<dyn ChunkingIndexProvider>>) -> FileIndexer {
        FileIndexer { index_providers: providers }
    }
}

#[derive(Clone)]
pub struct FileQueryer<C>
where
    C: KeyedSequencedStore<String, QueryCursor> +
        ClearByFilter<QueryCursor> +
        Send + Sync
{
    index_providers: Vec<Arc<dyn ChunkingIndexProvider>>,
    cursor_store: C,
}

impl<C> FileQueryer<C>
where
    C: KeyedSequencedStore<String, QueryCursor> +
        ClearByFilter<QueryCursor> +
        Send + Sync
{
    // Testing constructor
    async fn new() -> Result<FileQueryer<LanceDBStore<QueryCursor>>, FileIndexError> {
        let cursor_store = LanceDBStore::local("./data_dir", "cursor_index".to_owned()).await
            .map_err(|e| FileIndexError::DependencyError {
                dependency: "Lance Db Vector Store", 
                source: Box::new(e)
            })?;

        let basic_image = BasicImageIndexProvider::using(
            LanceDBStore::local("./data_dir", "basic_image_index".to_owned()).await
            .map_err(|e| FileIndexError::DependencyError {
                dependency: "Lance Db Vector Store", 
                source: Box::new(e)
            })?,
        );

        Ok(FileQueryer::with(vec![Arc::new(basic_image)], cursor_store))
    }

    pub fn with(providers: Vec<Arc<dyn ChunkingIndexProvider>>, cursor_store: C) -> FileQueryer<C> {
        FileQueryer { index_providers: providers, cursor_store }
    }
}

pub mod index_files;
pub mod pagination;
pub mod query_files;