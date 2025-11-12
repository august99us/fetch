use std::{error::Error, future::Future, sync::Arc};

use tokio::task::JoinSet;

use crate::{files::pagination::QueryCursor, index::provider::{ChunkingIndexProvider, image::ImageIndexProvider}, store::{ClearByFilter, KeyedSequencedStore, lancedb::LanceDBStore}};

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
    #[allow(dead_code)]
    async fn new() -> Result<FileIndexer, FileIndexError> {
        let basic_image = ImageIndexProvider::using(Arc::new(
            LanceDBStore::local("./data_dir", "basic_image_index".to_owned()).await
            .map_err(|e| FileIndexError::DependencyError {
                dependency: "Lance Db Vector Store", 
                source: Box::new(e)
            })?,
        ));

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
    #[allow(dead_code)]
    async fn new() -> Result<FileQueryer<LanceDBStore<QueryCursor>>, FileIndexError> {
        let cursor_store = LanceDBStore::local("./data_dir", "cursor_index".to_owned()).await
            .map_err(|e| FileIndexError::DependencyError {
                dependency: "Lance Db Vector Store", 
                source: Box::new(e)
            })?;

        let basic_image = ImageIndexProvider::using(Arc::new(
            LanceDBStore::local("./data_dir", "basic_image_index".to_owned()).await
            .map_err(|e| FileIndexError::DependencyError {
                dependency: "Lance Db Vector Store", 
                source: Box::new(e)
            })?,
        ));

        Ok(FileQueryer::with(vec![Arc::new(basic_image)], cursor_store))
    }

    pub fn with(providers: Vec<Arc<dyn ChunkingIndexProvider>>, cursor_store: C) -> FileQueryer<C> {
        FileQueryer { index_providers: providers, cursor_store }
    }
}

#[allow(async_fn_in_trait)]
pub trait ChunkingIndexProviderConcurrent {
    async fn distribute_calls<F, Fut, R>(&self, func: F) -> Result<Vec<R>, anyhow::Error>
    where
        // Would use AsyncFn (which is FnOnce) but being able to specify that the generated
        // Future is Send requires unstable features (access to the AsyncFn::CallRefFuture
        // internal type)
        // This Fn must be clone so that it can be passed to multiple tasks running
        // concurrently.
        F: (FnOnce(Arc<dyn ChunkingIndexProvider>) -> Fut) + Clone + Send + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: Send + 'static;
}

impl ChunkingIndexProviderConcurrent for Vec<Arc<dyn ChunkingIndexProvider>> {
    async fn distribute_calls<F, Fut, R>(&self, func: F) -> Result<Vec<R>, anyhow::Error>
    where
        F: (FnOnce(Arc<dyn ChunkingIndexProvider>) -> Fut) + Clone + Send + 'static,
        Fut: Future<Output = R> + Send + 'static,
        R: Send + 'static,
    {
        let mut joinset = JoinSet::new();
        for provider in self {
            let provider_clone = provider.clone();
            let fn_clone = func.clone();
            joinset.spawn(async move {
                fn_clone(provider_clone).await
            });
        }

        let mut results = Vec::with_capacity(self.len());
        while let Some(res) = joinset.join_next().await {
            match res {
                Ok(res) => {
                    results.push(res);
                },
                Err(e) => {
                    return Err(anyhow::anyhow!("Join error occurred while distributing calls: {}", e));
                }
            }
        }

        Ok(results)
    }
}

pub mod index;
pub mod pagination;
pub mod query;