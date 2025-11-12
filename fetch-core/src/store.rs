use std::future::Future;

use chrono::{DateTime, Utc};
use serde::Serialize;

/// Errors that can occur during keyed store operations.
#[derive(thiserror::Error, Debug)]
pub enum KeyedSequencedStoreError {
    #[error("Error while serializing element for storage")]
    Serialization { element: String, #[source] source: anyhow::Error },
    #[error("Error performing Put operation")]
    Put { issue: &'static str, #[source] source: anyhow::Error },
    #[error("Error performing Clear operation")]
    Clear { issue: &'static str, #[source] source: anyhow::Error },
    #[error("Error performing Get operation")]
    Get { issue: &'static str, #[source] source: anyhow::Error },
    #[error("Unknown Error")]
    Other { #[source] source: anyhow::Error },
}

// Base traits
pub trait KeyedSequencedStore<K: Serialize + Send, D: KeyedSequencedData<K>> {
    fn put(&self, data: Vec<D>) -> impl Future<Output = Result<(), KeyedSequencedStoreError>> + Send;
    fn clear(&self, key: K, optional_sequence_number: Option<u64>) -> impl Future<Output = Result<(), KeyedSequencedStoreError>> + Send;
    fn get(&self, key: K) -> impl Future<Output = Result<Option<D>, KeyedSequencedStoreError>> + Send;
}

pub trait KeyedSequencedData<K: Serialize + Send> {
    fn get_key(&self) -> K;
    fn get_sequence_num(&self) -> u64;
}

// Filter traits

#[derive(thiserror::Error, Debug)]
pub enum FilterStoreError {
    #[error("Filter provided for attribute that is not marked as filterable")]
    UnavailableFilter { attribute: String },
    /// An error occurred during a CRUD operation on a single record.
    /// 
    /// This error wraps underlying storage errors that occur during index, update,
    /// or delete operations on individual records.
    #[error("Error performing clear with filters operation on table")]
    Clear { #[source] source: anyhow::Error },
    #[error("Error performing query with filters on table")]
    Query { #[source] source: anyhow::Error },
    #[error("Unknown Error")]
    Other { #[source] source: anyhow::Error },
}

pub trait Filterable {
    fn filterable_attributes() -> Vec<&'static str>;
}

pub enum FilterRelation {
    Lt,
    Eq,
    Gt,
}

pub struct Filter<'a> {
    pub attribute: &'a str,
    pub filter: FilterValue<'a>,
    pub relation: FilterRelation,
}

pub enum FilterValue<'a> {
    String(&'a str),
    Int(i32),
    Float(f32),
    DateTime(&'a DateTime<Utc>),
}

pub trait ClearByFilter<D: Filterable> {
    fn clear_filter<'a>(&self, filters: &[Filter<'a>]) -> impl Future<Output = Result<(), FilterStoreError>> + Send;
}

pub trait QueryByFilter<D: Filterable> {
    fn query_filter<'a>(&self, filters: &[Filter<'a>]) -> impl Future<Output = Result<Vec<D>, FilterStoreError>> + Send;
    fn query_filter_n<'a>(&self, filters: &[Filter<'a>], num_results: u32, offset: u32) -> impl Future<Output = Result<Vec<D>, FilterStoreError>> + Send;
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
    fn query_vector(&self, vector: Vec<f32>) -> impl Future<Output = Result<Vec<VectorQueryResult<D>>, VectorStoreError>> + Send;
    fn query_vector_n(&self, vector: Vec<f32>, num_results: u32, offset: u32) ->
        impl Future<Output = Result<Vec<VectorQueryResult<D>>, VectorStoreError>> + Send;
}

pub struct VectorQueryResult<D: VectorData> {
    pub result: D,
    /// Ascending distance score from the query vector. Lower = better
    pub distance: f32,
}

pub trait FTSData {
    fn fts_attributes() -> Vec<&'static str>;
}

pub trait QueryFull<D: VectorData + Filterable + FTSData> {
    fn query_full<'a>(&self, vector: Vec<f32>, fts_terms: Option<&str>, filters: &[Filter<'a>]) -> 
        impl Future<Output = Result<Vec<FullQueryResult<D>>, anyhow::Error>> + Send;
    fn query_full_n<'a>(
        &self,
        vector: Vec<f32>,
        fts_terms: Option<&str>,
        filters: &[Filter<'a>],
        num_results: u32,
        offset: u32,
    ) -> impl Future<Output = Result<Vec<FullQueryResult<D>>, anyhow::Error>> + Send;
}

pub struct FullQueryResult<D: VectorData + Filterable + FTSData> {
    pub result: D,
    /// Descending relevancy score from combined query factors. higher = better
    pub score: f32,
}

pub mod lancedb;