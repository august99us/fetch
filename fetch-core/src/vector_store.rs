use std::future::Future;

/// Errors that can occur during vector store operations.
#[derive(thiserror::Error, Debug)]
pub enum VectorStoreError {
    /// The provided vector has an invalid length for this vector store.
    /// 
    /// This error occurs when trying to index or query with a vector that doesn't
    /// match the expected dimensionality of the vector store.
    #[error("Invalid input vector length {inputted_vector_len:?}")]
    InvalidVectorLength { inputted_vector_len: u32, required_vector_len: u32 },
    
    /// An error occurred during a CRUD operation on a single record.
    /// 
    /// This error wraps underlying storage errors that occur during index, update,
    /// or delete operations on individual records.
    #[error("Error performing crud operation on single record in index with key {record_key:?}")]
    RecordOperation { record_key: String, operation: &'static str, #[source] source: anyhow::Error },
    
    /// An error occurred during vector query execution.
    /// 
    /// This error wraps underlying errors that occur during similarity search operations.
    #[error("Error performing vector query")]
    Query { #[source] source: anyhow::Error }
}

/// Describes an object that understands how to store vectors in an optimistically locked manner.
/// 
/// This trait provides methods for storing and deleting vector records with support for
/// sequence-based ordering to handle potentially out-of-order operations.
pub trait IndexVector {
    /// Index a vector, creating the record or updating it if it exists.
    /// 
    /// The sequence number is used to validate ordering between possibly out-of-order events,
    /// ensuring that newer updates don't get overwritten by older ones.
    /// 
    /// # Arguments
    /// 
    /// * `key` - A unique identifier for this vector record
    /// * `vector` - The vector data to store
    /// * `sequence_number` - A sequence number for ordering operations
    /// 
    /// # Returns
    /// 
    /// Returns `Ok(())` if the indexing succeeded, or a `VectorStoreError` if it failed.
    fn index(&self, key: &str, vector: Vec<f32>, sequence_number: u64) -> impl Future<Output = Result<(), VectorStoreError>> + Send;
    
    /// Delete a vector record from the store.
    /// 
    /// An optional sequence number can be provided for optimistic locking to handle
    /// potentially out-of-order delete operations.
    /// 
    /// # Arguments
    /// 
    /// * `key` - The unique identifier of the record to delete
    /// * `optional_sequence_number` - Optional sequence number for optimistic locking
    /// 
    /// # Returns
    /// 
    /// Returns `Ok(())` if the deletion succeeded, or a `VectorStoreError` if it failed.
    fn delete(&self, key: &str, optional_sequence_number: Option<u64>) -> impl Future<Output = Result<(), VectorStoreError>> + Send;
}

/// Result of a vector similarity query containing the key and distance.
/// 
/// Note: The specific distance metric used (L2, cosine, etc) is currently an implementation detail 
/// and is not guaranteed as part of the API contract, but may be standardized in the future.
#[derive(Debug)]
pub struct QueryKeyResult {
    /// The unique key identifier of the similar vector
    pub key: String,
    /// The distance measure indicating how similar this vector is to the query vector
    pub distance: f32,
}

/// Describes an object that understands how to perform similarity queries against stored vectors.
/// 
/// This trait provides methods for finding vectors that are similar to a given query vector,
/// returning the keys of matching records along with their similarity distances.
pub trait QueryVectorKeys {
    /// Query for similar vectors and return their keys with similarity distances.
    /// 
    /// Returns a default number of results determined by the implementation.
    /// 
    /// # Arguments
    /// 
    /// * `vector` - The query vector to find similarities for
    /// 
    /// # Returns
    /// 
    /// A vector of `QueryKeyResult` containing similar vectors and their distances,
    /// or a `VectorStoreError` if the query failed.
    async fn query_keys(&self, vector: Vec<f32>) -> Result<Vec<QueryKeyResult>, VectorStoreError>;
    
    /// Query for a specific number of similar vectors and return their keys with similarity distances.
    /// 
    /// # Arguments
    /// 
    /// * `vector` - The query vector to find similarities for
    /// * `num_results` - The maximum number of results to return
    /// 
    /// # Returns
    /// 
    /// A vector of `QueryKeyResult` containing similar vectors and their distances,
    /// or a `VectorStoreError` if the query failed.
    async fn query_n_keys(&self, vector: Vec<f32>, num_results: u32) -> Result<Vec<QueryKeyResult>, VectorStoreError>;
}

pub mod lancedb_store;