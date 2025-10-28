use serde::Serialize;

/// Errors that can occur during keyed store operations.
#[derive(thiserror::Error, Debug)]
pub enum KeyedSequencedStoreError {
    #[error("Error while serializing element for storage")]
    Serialization { element: String, #[source] source: anyhow::Error },
    /// An error occurred during a CRUD operation on a single record.
    /// 
    /// This error wraps underlying storage errors that occur during index, update,
    /// or delete operations on individual records.
    #[error("Error performing CRUD operation on table")]
    RecordOperation { operation: &'static str, #[source] source: anyhow::Error },
    #[error("Unknown Error")]
    Other { #[source] source: anyhow::Error },
}

// Base traits

pub trait KeyedSequencedStore<K: Serialize, D: KeyedSequencedData<K>> {
    async fn put(&self, data: Vec<D>) -> Result<(), KeyedSequencedStoreError>;
    async fn clear(&self, key: Vec<K>) -> Result<(), KeyedSequencedStoreError>;
    async fn get(&self, key: K) -> Result<D, KeyedSequencedStoreError>;
}

pub trait KeyedSequencedData<K> {
    fn get_key(&self) -> K;
    fn get_sequence_num(&self) -> u64;
}

// Filter traits

pub trait Filterable {
    fn filterable_attribute_names() -> Vec<String>;
}

pub enum FilterRelation {
    Lt,
    Eq,
    Gt,
}

pub struct Filter {
    pub attribute: String,
    pub filter: String,
    pub relation: FilterRelation,
}

pub trait ClearByFilter<D: Filterable> {
    fn clear_filter(&self, filters: Vec<Filter>);
}

pub trait QueryByFilter<D: Filterable> {
    fn query_filter(&self, filters: Vec<Filter>);
    fn query_filter_n(&self, filters: Vec<Filter>, num_results: u32, offset: u32);
}

// Vector traits

/// Errors that can occur during vector store operations.
#[derive(thiserror::Error, Debug)]
pub enum VectorStoreError {
    /// The provided vector has an invalid length for this vector store.
    /// 
    /// This error occurs when trying to query with a vector that doesn't
    /// match the expected dimensionality of the vector store.
    #[error("Invalid input vector length {inputted_vector_len:?}")]
    InvalidVectorLength { inputted_vector_len: u32, required_vector_len: u32 },
    /// An error occurred during vector query execution.
    /// 
    /// This error wraps underlying errors that occur during similarity search operations.
    #[error("Error performing vector query")]
    Query { #[source] source: anyhow::Error }
}

pub trait VectorData {
    fn get_vector(&self) -> &[f32];
    fn vector_attribute() -> &'static str;
    fn vector_length() -> u32;
}

pub trait QueryByVector<D: VectorData> {
    async fn query_vector(&self, vector: Vec<f32>) -> Result<Vec<VectorQueryResult<D>>, VectorStoreError>;
    async fn query_vector_n(&self, vector: Vec<f32>, num_results: u32, offset: u32) -> Result<Vec<VectorQueryResult<D>>, VectorStoreError>;
}

pub struct VectorQueryResult<D: VectorData> {
    pub result: D,
    pub distance: f32,
}

pub mod lancedb;