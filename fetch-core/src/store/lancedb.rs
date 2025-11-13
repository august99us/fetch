use std::{future::Future, marker::PhantomData, sync::{Arc, LazyLock, atomic::{AtomicI32, Ordering}}};

use arrow::array::{StringBuilder, UInt64Builder};
use arrow_array::{Array, ArrayRef, Float32Array, RecordBatch, RecordBatchIterator, RecordBatchReader, StructArray};
use arrow_schema::{DataType, Field, Schema};
use futures::stream::StreamExt;
use lancedb::{Connection, DistanceType, Table, connect, database::CreateTableMode, index::{Index, scalar::{FtsQuery, FullTextSearchQuery, MultiMatchQuery}}, query::{ExecutableQuery, Query, QueryBase, QueryExecutionOptions, VectorQuery}, rerankers::{Reranker, rrf::RRFReranker}, table::OptimizeAction};
use log::info;
use serde::Serialize;

use crate::store::{ClearByFilter, FTSData, Filter, FilterRelation, FilterStoreError, FilterValue, Filterable, FullQueryResult, KeyedSequencedData, KeyedSequencedStore, KeyedSequencedStoreError, QueryByFilter, QueryByVector, QueryFull, VectorData, VectorQueryResult, VectorStoreError};

// Number of operations to run before running optimize.
const OPERATIONS_PER_OPTIMIZE: i32 = 20;

#[derive(thiserror::Error, Debug)]
pub enum LanceDBError {
    #[error("Error while performing merge insert operation")]
    MergeInsert { #[source] source: lancedb::error::Error },
    #[error("Error while performing delete operation")]
    Delete { #[source] source: lancedb::error::Error },
    #[error("Error while optimizing table")]
    Optimize { original_operation: &'static str, #[source] source: lancedb::error::Error },
    #[error("Invalid parameters provided to constructor")]
    InvalidParameter { parameter: &'static str, issue: &'static str, #[source] source: Option<anyhow::Error> },
    #[error("Issue creating connection to data directory")]
    Connection (#[source] lancedb::error::Error),
    #[error("Error performing holistic table operations")]
    TableOperation { operation: &'static str, #[source] source: lancedb::error::Error },
}

pub trait ArrowData: Send + Sync where Self: Sized {
    type RowBuilder: RowBuilder<Self> + Send;

    fn schema() -> Schema;
    fn row_builder() -> Self::RowBuilder;
    fn attribute_to_column_name(attr: &str) -> &'static str;
    fn batch_to_iter(record_batch: RecordBatch) -> impl IntoIterator<Item = Self>;
}

pub trait RowBuilder<D> {
    fn append(&mut self, row: D);

    /// Note: returns StructArray to allow nesting within another array if desired
    /// Fields must be ordered in the same way as in the ArrowData::schema()
    fn finish(self) -> Vec<(Arc<Field>, ArrayRef)>;
}
impl<D> Extend<D> for dyn RowBuilder<D> {
    fn extend<T: IntoIterator<Item = D>>(&mut self, iter: T) {
        iter.into_iter().for_each(|row| self.append(row));
    }
}

/// LanceDB store that works with ArrowData types.
/// Additional functionality is available based on trait bounds:
/// - D: VectorData enables vector query methods via local_vector()
/// - D: Filterable enables filter/index methods (future)
#[derive(Clone)]
pub struct LanceDBStore<D: ArrowData> {
    db: Connection,
    table: Table,
    table_name: String,
    schema: Arc<Schema>,
    ops_to_optimize: Arc<AtomicI32>,
    _phantom_data: PhantomData<D>,
}

// development function to clear all the data from a given directory with LanceDB data inside
pub async fn drop(data_dir: &str, table_name: &str) -> Result<(), LanceDBError> {
    let db = connect(data_dir)
        .execute().await
        .map_err(LanceDBError::Connection)?;
    db.drop_table(table_name).await
        .map_err(|e| LanceDBError::TableOperation { operation: "Dropping table", source: e })?;
    Ok(())
}

impl<D: ArrowData> LanceDBStore<D> {
    pub async fn local(data_dir: &str, table_name: String) -> Result<LanceDBStore<D>, LanceDBError> {
        let extended_schema = D::schema();

        let base_schema = build_base_schema();
        let schema = Arc::new(Schema::try_merge([base_schema, extended_schema])
            .map_err(|e| LanceDBError::InvalidParameter {
                parameter: "data schema",
                issue: "Data schema and base schema could not be merged. \
                    Could there be a key conflict? Data schema must not use 'key' or 'sequence_number' keys.",
                source: Some(e.into()),
            })?);

        let db = connect(data_dir)
            .execute().await
            .map_err(LanceDBError::Connection)?;
        let table = db.create_empty_table(table_name.clone(), schema.clone())
            .mode(CreateTableMode::ExistOk(Box::new(|r| r)))
            .execute().await
            .map_err(|e| LanceDBError::TableOperation { operation: "Creating or opening table", source: e })?;

        Self::create_key_index(&table).await?;

        Ok(LanceDBStore {
            db,
            table,
            table_name,
            schema,
            ops_to_optimize: Arc::new(AtomicI32::new(OPERATIONS_PER_OPTIMIZE)),
            _phantom_data: Default::default(),
        })
    }
    
    // development function to clear all the data from an instantiated LanceDBStore
    pub async fn clear_all(&mut self) -> Result<(), LanceDBError> {
        self.db.drop_table(self.table_name.clone()).await
            .map_err(|e| LanceDBError::TableOperation { operation: "Dropping all tables", source: e })?;
        self.table = self.db.create_empty_table(self.table_name.clone(), self.schema.clone())
            .mode(CreateTableMode::ExistOk(Box::new(|r| r)))
            .execute().await
            .map_err(|e| LanceDBError::TableOperation { operation: "Creating table", source: e })?;
        Ok(())
    }

    pub async fn merge_insert(&self, reader: impl RecordBatchReader + Send + 'static) -> Result<(), LanceDBError> {
        let mut merge = self.table.merge_insert(&[KEY_COLUMN]);
        merge.when_matched_update_all(Some(format!("target.{SEQUENCE_NUMBER_COLUMN} < \
            source.{SEQUENCE_NUMBER_COLUMN}"))).when_not_matched_insert_all();

        merge.execute(Box::new(reader)).await
            .map_err(|e| LanceDBError::MergeInsert { source: e })?;

        self.maybe_optimize().await
    }

    pub async fn delete_one(&self, key: String, optional_sequence_number: Option<u64>) -> Result<(), LanceDBError> {
        let mut delete_condition = format!("{KEY_COLUMN} = '{key}'");
        if let Some(sn) = optional_sequence_number {
            delete_condition.push_str(&format!(" AND {SEQUENCE_NUMBER_COLUMN} < {sn}"));
        }

        self.table.delete(&delete_condition).await
            .map_err(|e| LanceDBError::Delete { source: e })?;

        self.maybe_optimize().await
    }

    /// TODO: documentation
    /// It is recommended to call this function after every table record operation that is performed.
    async fn maybe_optimize(&self) -> Result<(), LanceDBError> {
        // Atomically decrement the counter and get the previous value
        let prev_count = self.ops_to_optimize.fetch_sub(1, Ordering::Relaxed);

        // If the previous count was <= 1, it means after decrement it's <= 0
        // so we should optimize and reset the counter
        if prev_count <= 1 {
            // Reset the counter immediately to reduce the probability of multiple threads
            // both triggering optimization
            self.ops_to_optimize.store(OPERATIONS_PER_OPTIMIZE, Ordering::Relaxed);

            info!("Optimizing table: {}", self.table_name);
            // Run optimization (this may take a while, but counter is already reset)
            self.table.optimize(OptimizeAction::All).await
                .map_err(|e| LanceDBError::Optimize { original_operation: "merge_insert", source: e })?;
        }
        Ok(())
    }

    /// Creates index on key column, allowing for key based retrievals
    async fn create_key_index(table: &Table) -> Result<(), LanceDBError> {
        info!("Table {}: Creating key index", table.name());

        table.create_index(&[KEY_COLUMN], Index::BTree(Default::default()))
            .execute()
            .await
            .map_err(|e| LanceDBError::TableOperation {
                operation: "Creating key index",
                source: e
            })?;

        Ok(())
    }
}

// Base implementation on LanceDBStore - no VectorData requirement
impl<K: Serialize + Send, D: ArrowData + KeyedSequencedData<K>> KeyedSequencedStore<K, D> for LanceDBStore<D> {
    async fn put(&self, data: Vec<D>) -> Result<(), KeyedSequencedStoreError> {
        let mut key_array = StringBuilder::new();
        let mut row_builder = D::row_builder();
        let mut sequence_array = UInt64Builder::new();
        for arrow_data in data {
            key_array.append_value(serde_json::to_string(&arrow_data.get_key()).map_err(|e|
                KeyedSequencedStoreError::Serialization { element: "key".to_owned(), source: e.into() })?);
            sequence_array.append_value(arrow_data.get_sequence_num());
            row_builder.append(arrow_data);
        }

        // These fields must be ordered in the same way as the schema
        let mut data_columns = vec![
            (KEY_FIELD.clone(), Arc::new(key_array.finish()) as ArrayRef),
            (SEQUENCE_NUMBER_FIELD.clone(), Arc::new(sequence_array.finish()) as ArrayRef),
        ];
        for field_and_array in row_builder.finish() {
            data_columns.push(field_and_array)
        }

        let struct_array = StructArray::from(data_columns);

        // push the data
        let reader = RecordBatchIterator::new(
            vec![RecordBatch::from(struct_array)]
                .into_iter()
                .map(Ok),
            self.schema.clone(),
        );

        self.merge_insert(reader).await
            .map_err(|e| KeyedSequencedStoreError::Put { issue: "merge_insert", source: e.into() })
    }

    async fn clear(&self, key: K, optional_sequence_number: Option<u64>) -> Result<(), KeyedSequencedStoreError> {
        let key_string = serde_json::to_string(&key).map_err(|e|
                KeyedSequencedStoreError::Serialization { element: "key".to_owned(), source: e.into() })?;

        self.delete_one(key_string, optional_sequence_number).await
            .map_err(|e| KeyedSequencedStoreError::Clear { issue: "delete_one", source: e.into() })
    }

    async fn get(&self, key: K) -> Result<Option<D>, KeyedSequencedStoreError> {
        let key_string = serde_json::to_string(&key).map_err(|e|
                KeyedSequencedStoreError::Serialization { element: "key".to_owned(), source: e.into() })?;

        let mut query = self.table.query();
        query = apply_key_filter(query, &key_string);

        let mut result_stream = query.execute().await
            .map_err(|e| KeyedSequencedStoreError::Get { issue: "query execution", source: e.into() })?;

        let mut result_list: Vec<D> = Vec::new();
        while let Some(rb) = result_stream.next().await {
            let batch = rb.map_err(|e| KeyedSequencedStoreError::Get { issue: "read RecordBatch", source: e.into() })?;

            for item in D::batch_to_iter(batch) {
                result_list.push(item);
            }
        }

        match result_list.len() {
            0 => Ok(None),
            1 => Ok(result_list.pop()),
            _ => Err(KeyedSequencedStoreError::Get {
                issue: "Multiple records found for key", 
                source: anyhow::Error::msg("non-unique key"),
            }),
        }
    }
}

// Vector-specific methods, only available when D: VectorData
impl<D: ArrowData + VectorData> LanceDBStore<D> {
    /// Creates a LanceDBStore with vector validation.
    /// This validates that the vector length can be safely cast to i32 for Arrow compatibility.
    pub async fn local_vector(data_dir: &str, table_name: String) -> Result<LanceDBStore<D>, LanceDBError> {
        let vector_len = D::vector_length();
        // If this cast (usize -> i32) does not work, then there will be issues interacting with Arrow later.
        // This is an implementation detail and may change in the future.
        // Very rarely do i expect this not to work but performing the cast once here will expose the issue
        // earlier in the process of using this class.
        TryInto::<i32>::try_into(vector_len).map_err(|e| LanceDBError::InvalidParameter {
            parameter: "vector length",
            issue: "vector length could not be cast into an i32. could the number be too large for i32?",
            source: Some(e.into()),
        })?;

        Self::local(data_dir, table_name).await
    }
}

// Filterable-specific methods, only available when D: Filterable
impl<D: ArrowData + Filterable> LanceDBStore<D> {
    /// Creates a LanceDBStore with indexes on filterable attributes.
    pub async fn local_with_filters(data_dir: &str, table_name: String) -> Result<LanceDBStore<D>, LanceDBError> {
        let store = Self::local(data_dir, table_name).await?;
        store.create_filter_indexes().await?;
        Ok(store)
    }

    /// Clears all data and recreates the table with indexes.
    pub async fn clear_all_with_filters(&mut self) -> Result<(), LanceDBError> {
        self.clear_all().await?;
        self.create_filter_indexes().await?;
        Ok(())
    }

    /// Creates indexes on all filterable attributes.
    async fn create_filter_indexes(&self) -> Result<(), LanceDBError> {
        let attribute_names = D::filterable_attributes();
        let column_names: Vec<&str> = attribute_names.iter()
            .map(|attr| D::attribute_to_column_name(attr))
            .collect();

        if !column_names.is_empty() {
            info!("Table {}: Creating filter indexes on columns: {:?}", self.table_name, column_names);

            for column_name in column_names {
                self.table.create_index(&[column_name], Index::BTree(Default::default()))
                    .execute()
                    .await
                    .map_err(|e| LanceDBError::TableOperation {
                        operation: "Creating filter indexes",
                        source: e
                    })?;
            }
        }

        Ok(())
    }
}

// FTSData-specific methods, only available when D: FTSData
impl<D: ArrowData + FTSData> LanceDBStore<D> {
    /// Creates a LanceDBStore with FTS indexes on FTS attributes.
    pub async fn local_with_fts(data_dir: &str, table_name: String) -> Result<LanceDBStore<D>, LanceDBError> {
        let store = Self::local(data_dir, table_name).await?;
        store.create_fts_indexes().await?;
        Ok(store)
    }

    /// Clears all data and recreates the table with FTS indexes.
    pub async fn clear_all_with_fts(&mut self) -> Result<(), LanceDBError> {
        self.clear_all().await?;
        self.create_fts_indexes().await?;
        Ok(())
    }

    /// Creates FTS indexes on all FTS attributes.
    async fn create_fts_indexes(&self) -> Result<(), LanceDBError> {
        let attribute_names = D::fts_attributes();
        let column_names: Vec<&str> = attribute_names.iter()
            .map(|attr| D::attribute_to_column_name(attr))
            .collect();

        if !column_names.is_empty() {
            info!("Table {}: Creating FTS indexes on columns: {:?}", self.table_name, column_names);

            for column_name in column_names {
                self.table.create_index(&[column_name], Index::FTS(Default::default()))
                    .execute()
                    .await
                    .map_err(|e| LanceDBError::TableOperation {
                        operation: "Creating FTS indexes",
                        source: e
                    })?;
            }
        }

        Ok(())
    }
}

// Combined: Vector + Filterable + FTSData
impl<D: ArrowData + VectorData + Filterable + FTSData> LanceDBStore<D> {
    /// Creates a LanceDBStore with vector validation, filterable indexes, and FTS indexes.
    pub async fn local_full(data_dir: &str, table_name: String) -> Result<LanceDBStore<D>, LanceDBError> {
        let store = Self::local_vector(data_dir, table_name).await?;
        store.create_filter_indexes().await?;
        store.create_fts_indexes().await?;
        Ok(store)
    }

    /// Clears all data and recreates the table with all indexes.
    pub async fn clear_all_full(&mut self) -> Result<(), LanceDBError> {
        self.clear_all().await?;
        self.create_filter_indexes().await?;
        self.create_fts_indexes().await?;
        Ok(())
    }
}

// ClearByFilter implementation - only available when D: Filterable
impl<D: ArrowData + Filterable> ClearByFilter<D> for LanceDBStore<D> {
    async fn clear_filter<'a>(&self, filters: &[Filter<'a>]) -> Result<(), FilterStoreError> {
        if filters.is_empty() {
            return Ok(());
        }

        let condition = build_filter_condition::<D>(filters)?;

        self.table.delete(&condition).await
            .map_err(|e| FilterStoreError::Clear { source: e.into() })?;

        self.maybe_optimize().await
            .map_err(|e| FilterStoreError::Clear { source: e.into() })?;

        Ok(())
    }
}

// QueryByFilter implementation - only available when D: Filterable
impl<D: ArrowData + Filterable> QueryByFilter<D> for LanceDBStore<D> {
    fn query_filter<'a>(&self, filters: &[Filter<'a>]) -> impl Future<Output = Result<Vec<D>, FilterStoreError>> {
        self.query_filter_n(filters, 0, 0)
    }

    async fn query_filter_n<'a>(&self, filters: &[Filter<'a>], num_results: u32, offset: u32) -> Result<Vec<D>, FilterStoreError> {
        let mut query = self.table.query();
        query = apply_filters::<D, _>(query, filters)?;
        query = apply_pagination(query, num_results, offset);

        let mut result_stream = query.execute().await
            .map_err(|e| FilterStoreError::Query { source: e.into() })?;

        let mut result_list: Vec<D> = Vec::new();
        while let Some(rb) = result_stream.next().await {
            let batch = rb.map_err(|e| FilterStoreError::Query { source: e.into() })?;

            for item in D::batch_to_iter(batch) {
                result_list.push(item);
            }
        }

        Ok(result_list)
    }
}

// QueryByVector implementation - only available when D: VectorData
impl<D: ArrowData + VectorData> QueryByVector<D> for LanceDBStore<D> {
    fn query_vector(&self, vector: Vec<f32>) -> impl Future<Output = Result<Vec<VectorQueryResult<D>>, VectorStoreError>> {
        self.query_vector_n(vector, 0, 0)
    }

    async fn query_vector_n(&self, vector: Vec<f32>, num_results: u32, offset: u32) -> Result<Vec<VectorQueryResult<D>>, VectorStoreError> {
        let mut query = self.table.query();
        query = apply_pagination(query, num_results, offset);
        let query = apply_vector_search::<D>(query, vector)?;

        let mut result_stream = query.execute().await
            .map_err(|e| VectorStoreError::Query { source: e.into() })?;

        let mut result_list: Vec<VectorQueryResult<D>> = Vec::new();
        while let Some(rb) = result_stream.next().await {
            match rb {
                Ok(batch) => {
                    let distance_column = batch.column_by_name("_distance") // Pick out the distance column
                        .expect("_distance column should exist in vector query")
                        // cast to a float32 array
                        .as_any().downcast_ref::<Float32Array>()
                        // Expect: the column is definitively returned as a f32, this conversion from arrow array
                        // should not fail.
                        .expect("Returned query result of distances could not be converted to a f32")
                        // unwrap the optionals
                        // Expect: the column is defined as non-nullable in the schema, there should be no reason for
                        // the optional to be empty
                        .iter().map(|s| s.expect("Missing f32 in optional for non-nullable distance column"))
                        .collect::<Vec<f32>>();

                    let mut data_iter = D::batch_to_iter(batch).into_iter();
                    let mut distance_iter = distance_column.into_iter();

                    while let (Some(data), Some(distance)) = (data_iter.next(), distance_iter.next()) {
                        result_list.push(VectorQueryResult {
                            result: data,
                            distance,
                        })
                    }
                    if data_iter.next().is_some() || distance_iter.next().is_some() {
                        // TODO: probably better to return error here
                        panic!("columns in query result should not have different lengths!");
                    }
                }
                Err(e) => return Err(VectorStoreError::Query { source: e.into() })
            }
        }
        Ok(result_list)
    }
}

// QueryFull implementation - only available when D: VectorData + Filterable + FTSData
impl<D: ArrowData + VectorData + Filterable + FTSData> QueryFull<D> for LanceDBStore<D> {
    fn query_full<'a>(&self, vector: Vec<f32>, fts_terms: Option<&str>, filters: &[Filter<'a>]) ->
        impl Future<Output = Result<Vec<FullQueryResult<D>>, anyhow::Error>> {
        self.query_full_n(vector, fts_terms, filters, 0, 0)
    }

    async fn query_full_n<'a>(
        &self,
        vector: Vec<f32>,
        fts_terms: Option<&str>,
        filters: &[Filter<'a>],
        num_results: u32,
        offset: u32,
    ) -> Result<Vec<FullQueryResult<D>>, anyhow::Error> {
        let mut query = self.table.query();

        // Apply FTS
        if let Some(fts) = fts_terms {
            query = apply_fts::<D, _>(query, fts)?;
        }

        // Apply filters
        query = apply_filters::<D, _>(query, filters)
            .map_err(|e| VectorStoreError::Query { source: e.into() })?;

        // Apply pagination
        query = apply_pagination(query, num_results, offset);

        // Apply vector search
        let query = apply_vector_search::<D>(query, vector)?;

        // Execute hybrid search
        let mut result_stream = query.execute_hybrid(QueryExecutionOptions::default()).await
            .map_err(|e| VectorStoreError::Query { source: e.into() })?;

        let mut result_list: Vec<FullQueryResult<D>> = Vec::new();
        while let Some(rb) = result_stream.next().await {
            match rb {
                Ok(batch) => {
                    if batch.num_rows() == 0 {
                        // LanceDB will return a batch with num_rows = 0, but still containing the _score and
                        // _relevance_score columns (albeit empty) on a query with no results. However, none
                        // of the other columns will exist in the batch, so if we dont catch this empty result
                        // here, implementors of batch_to_iter may error out if they dont check for an empty
                        // result before they try pulling their non-existent columns out of the batch.
                        break;
                    }
                    let relevance_column = batch.column_by_name("_relevance_score")
                        .expect("_relevance_score column should exist in hybrid query")
                        .as_any().downcast_ref::<Float32Array>()
                        .expect("Returned query result of relevance scores could not be converted to a f32")
                        .iter().map(|s| s.expect("Missing f32 in optional for non-nullable relevance score column"))
                        .collect::<Vec<f32>>();

                    let mut data_iter = D::batch_to_iter(batch).into_iter();
                    let mut relevance_iter = relevance_column.into_iter();

                    while let (Some(data), Some(score)) = (data_iter.next(), relevance_iter.next()) {
                        result_list.push(FullQueryResult {
                            result: data,
                            score,
                        })
                    }
                    if data_iter.next().is_some() || relevance_iter.next().is_some() {
                        // TODO: probably better to return error here
                        panic!("columns in query result should not have different lengths!");
                    }
                }
                Err(e) => return Err(VectorStoreError::Query { source: e.into() }.into())
            }
        }
        Ok(result_list)
    }
}

// Private variables and methods

const KEY_COLUMN: &str = "key";
const SEQUENCE_NUMBER_COLUMN: &str = "sequence_number";

static KEY_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(KEY_COLUMN, DataType::Utf8, false))
});
static SEQUENCE_NUMBER_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(SEQUENCE_NUMBER_COLUMN, DataType::UInt64, false))
});
static DEFAULT_RERANKER: LazyLock<Arc<dyn Reranker>> = LazyLock::new(|| {
    Arc::new(RRFReranker::default())
});

/// Builds a base schema object given a number of floats that the embedded vector will occupy
/// This schema object should be merged with the data schema to make the full schema
fn build_base_schema() -> Schema {
    Schema::new(vec![
        KEY_FIELD.clone(),
        SEQUENCE_NUMBER_FIELD.clone(),
    ])
}

fn check_vector_length(given: u32, expected: u32) -> Result<(), VectorStoreError> {
    // check vector
    if given != expected {
        return Err(VectorStoreError::InvalidVectorLength {
            inputted_vector_len: given,
            required_vector_len: expected,
        });
    }
    Ok(())
}

// Helper function to apply exact match filter specifically for a key in the key column
// Keys should be guaranteed unique
fn apply_key_filter<Q: QueryBase>(query: Q, key: &str) -> Q {
    query.only_if(format!("{} = '{}'", KEY_COLUMN, key))
}

/// Builds a SQL WHERE condition from a list of filters.
/// Filters are combined with AND logic.
fn build_filter_condition<D: ArrowData + Filterable>(filters: &[Filter]) -> Result<String, FilterStoreError> {
    let filterable_attributes = D::filterable_attributes();

    let mut conditions = vec![];
    for filter in filters {
        if !filterable_attributes.contains(&filter.attribute) {
            return Err(FilterStoreError::UnavailableFilter { attribute: filter.attribute.to_owned() })
        }
        let column_name = D::attribute_to_column_name(filter.attribute);
        let operator = match filter.relation {
            FilterRelation::Lt => "<",
            FilterRelation::Eq => "=",
            FilterRelation::Gt => ">",
        };
        let condition_str = match filter.filter {
            FilterValue::String(s) => format!("{} {} '{}'", column_name, operator, s),
            FilterValue::Int(i) => format!("{} {} {}", column_name, operator, i),
            FilterValue::Float(f) => format!("{} {} {}", column_name, operator, f),
            FilterValue::DateTime(date_time) => format!(
                "{} {} timestamp '{}'",
                column_name,
                operator,
                date_time.format("%Y-%m-%d %H:%M:%S"),
            ),
        };
        conditions.push(condition_str);
    }

    Ok(conditions.join(" AND "))
}

/// Helper function to apply filters to a query if filters are not empty.
fn apply_filters<D: ArrowData + Filterable, Q: QueryBase>(mut query: Q, filters: &[Filter]) -> Result<Q, FilterStoreError> {
    if !filters.is_empty() {
        let condition = build_filter_condition::<D>(filters)?;
        query = query.only_if(&condition);
    }
    Ok(query)
}

/// Helper function to apply pagination (limit and offset) to a query.
fn apply_pagination<Q: QueryBase>(mut query: Q, num_results: u32, offset: u32) -> Q {
    if num_results > 0 {
        query = query.limit(num_results as usize);
    }
    if offset > 0 {
        query = query.offset(offset as usize);
    }
    query
}

/// Helper function to apply vector search parameters to a query.
fn apply_vector_search<D: ArrowData + VectorData>(query: Query, vector: Vec<f32>) -> Result<VectorQuery, VectorStoreError> {
    check_vector_length(vector.len() as u32, D::vector_length())?;

    let vector_column = D::attribute_to_column_name(D::vector_attribute());

    let query = query
        .nearest_to(vector)
        .expect("Unexpected issue converting Vec<f32> to QueryVector")
        .distance_type(DistanceType::Cosine)
        .column(vector_column);
    
    Ok(query)
}

fn apply_fts<D: ArrowData + FTSData, Q: QueryBase>(mut query: Q, fts_terms: &str) -> Result<Q, anyhow::Error> {
    let fts_columns: Vec<String> = D::fts_attributes()
        .into_iter()
        .map(|a| D::attribute_to_column_name(a).to_owned())
        .collect();
    if !fts_columns.is_empty() {
        let fts_query = FullTextSearchQuery::new_query(
            FtsQuery::MultiMatch(
                MultiMatchQuery::try_new(
                    fts_terms.to_string(),
                    fts_columns
                )?
            )
        );

        query = query.full_text_search(fts_query);
        query = query.rerank(DEFAULT_RERANKER.clone());
    }

    Ok(query)
}