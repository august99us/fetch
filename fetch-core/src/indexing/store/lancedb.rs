use std::{future::Future, marker::PhantomData, sync::{Arc, LazyLock, atomic::{AtomicI32, Ordering}}};

use arrow::array::{StringBuilder, UInt64Builder};
use arrow_array::{Array, ArrayRef, Float32Array, RecordBatch, RecordBatchIterator, RecordBatchReader, StructArray};
use arrow_schema::{DataType, Field, Schema};
use futures::stream::StreamExt;
use lancedb::{connect, database::CreateTableMode, query::{ExecutableQuery, QueryBase}, table::OptimizeAction, Connection, DistanceType, Table};
use log::info;
use serde::Serialize;

use crate::indexing::store::{KeyedSequencedData, KeyedSequencedStore, KeyedSequencedStoreError, QueryByVector, VectorData, VectorQueryResult};

use super::VectorStoreError;

// Number of operations to run before running optimize.
const OPERATIONS_PER_OPTIMIZE: i32 = 5;

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

pub trait ArrowData {
    type RowBuilder: RowBuilder<Self>;

    fn schema() -> Arc<Schema>;
    fn row_builder() -> Self::RowBuilder;
    fn attribute_to_column_name(attr: &'static str) -> &'static str;
    fn batch_to_iter(record_batch: RecordBatch) -> impl IntoIterator<Item = Self>;
}

pub trait RowBuilder<D> {
    fn append(&mut self, row: D);

    /// Note: returns StructArray to allow nesting within another array if desired
    fn finish(self) -> Vec<(Arc<Field>, ArrayRef)>;
}
impl<D> Extend<D> for dyn RowBuilder<D> {
    fn extend<T: IntoIterator<Item = D>>(&mut self, iter: T) {
        iter.into_iter().for_each(|row| self.append(row));
    }
}

/// Implements a vector storage index utilizing a LanceDB instance
/// under the hood.
#[derive(Clone)]
struct LanceDBVectorStore<D: ArrowData + VectorData> {
    db: Connection,
    table: Table,
    table_name: String,
    schema: Arc<Schema>,
    vector_len: u32,
    ops_to_optimize: Arc<AtomicI32>,
    _phantom_data: PhantomData<D>,
}

impl<D: ArrowData + VectorData> LanceDBVectorStore<D> {
    pub async fn local(data_dir: &str, table_name: String) -> Result<LanceDBVectorStore<D>, LanceDBError> {
        let extended_schema = (*D::schema()).clone();
        let vector_len = D::vector_length();
        // If this cast (usize -> i32) does not work, then there will be issues interacting with Arrow later.
        // This is an implementation detail and may change in the future.
        // Very rarely do i expect this not to work but performing the cast once here will expose the issue
        // earlier in the process of using this class.
        TryInto::<i32>::try_into(vector_len).map_err(|e| LanceDBError::InvalidParameter {
            parameter: "vector length",
            issue: "vector length could not be cast into an i32. could the number be too large for i32?",
            source: Some(e.into()),
        });

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
        Ok(LanceDBVectorStore {
            db,
            table,
            table_name,
            schema,
            vector_len,
            ops_to_optimize: Arc::new(AtomicI32::new(OPERATIONS_PER_OPTIMIZE)),
            _phantom_data: Default::default(),
        })
    }

    // development function to clear all the data from a given directory with LanceDB data inside
    pub async fn drop(data_dir: &str, table_name: String) -> Result<(), LanceDBError> {
        let db = connect(data_dir)
            .execute().await
            .map_err(LanceDBError::Connection)?;
        db.drop_table(table_name).await
            .map_err(|e| LanceDBError::TableOperation { operation: "Dropping all tables", source: e })?;
        Ok(())
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

    pub async fn delete(&self, key: String, optional_sequence_number: Option<u64>) -> Result<(), LanceDBError> {
        let mut delete_condition = format!("{KEY_COLUMN} = '{key}'");
        if let Some(sn) = optional_sequence_number {
            delete_condition.push_str(&format!(" AND {SEQUENCE_NUMBER_COLUMN} < {sn}"));
        }

        self.table.delete(&delete_condition).await
            .map_err(|e| LanceDBError::Delete { source: e })?;

        self.maybe_optimize().await
    }

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
}

impl<K: Serialize, D: ArrowData + KeyedSequencedData<K> + VectorData> KeyedSequencedStore<K, D> for LanceDBVectorStore<D> {
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

        let mut data_columns = vec![
            (KEY_FIELD.clone(), Arc::new(key_array.finish()) as ArrayRef)
        ];
        for field_and_array in row_builder.finish() {
            data_columns.push(field_and_array)
        }
        data_columns.push((SEQUENCE_NUMBER_FIELD.clone(), Arc::new(sequence_array.finish()) as ArrayRef));

        let struct_array = StructArray::from(data_columns);

        // push the data
        let reader = RecordBatchIterator::new(
            vec![RecordBatch::from(struct_array)]
                .into_iter()
                .map(Ok),
            self.schema.clone(),
        );

        self.merge_insert(reader).await
            .map_err(|e| KeyedSequencedStoreError::RecordOperation { operation: "put", source: e.into() })
    }

    async fn clear(&self, key: K, optional_sequence_number: Option<u64>) -> Result<(), KeyedSequencedStoreError> {
        let key_string = serde_json::to_string(&key).map_err(|e| 
                KeyedSequencedStoreError::Serialization { element: "key".to_owned(), source: e.into() })?;

        self.delete(key_string, optional_sequence_number).await
            .map_err(|e| KeyedSequencedStoreError::RecordOperation { operation: "clear", source: e.into() })
    }

    async fn get(&self, key: K) -> Result<D, KeyedSequencedStoreError> {
        todo!()
    }
}

impl<D: ArrowData + VectorData> QueryByVector<D> for LanceDBVectorStore<D> {
    fn query_vector(&self, vector: Vec<f32>) -> impl Future<Output = Result<Vec<VectorQueryResult<D>>, VectorStoreError>> {
        self.query_vector_n(vector, 0, 0)
    }

    async fn query_vector_n(&self, vector: Vec<f32>, num_results: u32, offset: u32) -> Result<Vec<VectorQueryResult<D>>, VectorStoreError> {
        check_vector_length(vector.len() as u32, D::vector_length());
        check_vector_length(self.vector_len, D::vector_length());

        let vector_column = D::attribute_to_column_name(D::vector_attribute());

        let query = self.table.query()
            // This normally returns errors because lancedb automatically uses an embedding model if registered
            // to convert a query into a vector. However without a registered model lancedb just expects the
            // actual vector to be provided here for the query, which is what I have done. Therefore this should
            // theoretically never cause an issue.
            .nearest_to(vector).expect("Unexpected issue converting Vec<f32> to QueryVector")
            .distance_type(DistanceType::Dot)
            .column(vector_column)
            // u32 -> usize casts, should always be fine
            .limit(num_results as usize)
            .offset(offset as usize);

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

                    while let (
                        Some(data),
                        Some(distance)
                    ) = (
                        data_iter.next(),
                        distance_iter.next()
                    ) {
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

// Private variables and methods

const KEY_COLUMN: &str = "key";
const SEQUENCE_NUMBER_COLUMN: &str = "sequence_number";

static KEY_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(KEY_COLUMN, DataType::Utf8, false))
});

static SEQUENCE_NUMBER_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(SEQUENCE_NUMBER_COLUMN, DataType::UInt64, false))
});

/// Builds a base schema object given a number of floats that the embedded vector will occupy
/// This schema object should be merged with the data schema to make the full schema
fn build_base_schema() -> Schema {
    Schema::new(vec![
        KEY_FIELD,
        SEQUENCE_NUMBER_FIELD,
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