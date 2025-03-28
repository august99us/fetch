use std::error::Error;

use crate::PreviewedFile;

#[derive(thiserror::Error, Debug)]
pub enum SemanticIndexError {
    #[error("Error calculating embedding for preview with path {record_key:?}")]
    PreviewEmbedding { record_key: String, #[source] source: Box<dyn Error> },
    #[error("Error performing crud operation on single record in index with path {record_key:?}")]
    RecordOperation { record_key: String, operation: &'static str, #[source] source: Box<dyn Error> },
    #[error("Error calculating embedding for query string {query:?}")]
    QueryEmbedding { query: String, #[source] source: Box<dyn Error> },
    #[error("Error performing vector query '{query:?}'")]
    Query { query: String, #[source] source: Box<dyn Error> }
}

/// index
pub trait IndexPreview {
    async fn index(&self, preview: PreviewedFile) -> Result<(), SemanticIndexError>;
    async fn delete(&self, path: &str) -> Result<(), SemanticIndexError>;
}

/// query and return path
pub trait QuerySimilarFiles {
    async fn query(&self, file_description: &str) -> Result<Vec<String>, SemanticIndexError>;
    async fn query_n(&self, file_description: &str, num_files: usize) -> Result<Vec<String>, SemanticIndexError>;
}

pub mod lancedb_store;