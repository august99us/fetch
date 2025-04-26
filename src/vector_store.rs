use std::future::Future;

#[derive(thiserror::Error, Debug)]
pub enum VectorStoreError {
    #[error("Invalid input vector length {inputted_vector_len:?}")]
    InvalidVectorLength { inputted_vector_len: u32, required_vector_len: u32 },
    #[error("Error performing crud operation on single record in index with key {record_key:?}")]
    RecordOperation { record_key: String, operation: &'static str, #[source] source: anyhow::Error },
    #[error("Error performing vector query")]
    Query { #[source] source: anyhow::Error }
}

/// index
pub trait IndexVector {
    /// Index a vector, creating the record or updating it if it exists, according to a provided sequence number
    /// which will be used to validate ordering between possibly out of order events
    fn index(&self, key: &str, vector: Vec<f32>, sequence_number: u64) -> impl Future<Output = Result<(), VectorStoreError>> + Send;
    /// Delete a vector record with an optional sequence number that can be used to optimistically lock on possibly
    /// out of order events
    fn delete(&self, key: &str, optional_sequence_number: Option<u64>) -> impl Future<Output = Result<(), VectorStoreError>> + Send;
}

#[derive(Debug)]
pub struct QueryKeyResult {
    pub key: String,
    pub distance: f32,
}
/// query and return path
pub trait QueryVectorKeys {
    async fn query_keys(&self, vector: Vec<f32>) -> Result<Vec<QueryKeyResult>, VectorStoreError>;
    async fn query_n_keys(&self, vector: Vec<f32>, num_results: u32) -> Result<Vec<QueryKeyResult>, VectorStoreError>;
}

pub mod lancedb_store;