use std::{future::Future, marker::PhantomData, sync::{Arc, LazyLock}};

use arrow::{array::{AsArray, FixedSizeListBuilder, Float32Builder, StringBuilder, UInt64Builder}, datatypes::Float32Type};
use arrow_array::{Array, ArrayRef, FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator, RecordBatchReader, StructArray};
use arrow_schema::{DataType, Field, Schema};
use futures::stream::StreamExt;
use lancedb::{connect, database::CreateTableMode, query::{ExecutableQuery, QueryBase}, table::merge::MergeInsertBuilder, Connection, DistanceType, Table};
use serde::Serialize;

use crate::indexing::store::{KeyedSequencedData, KeyedSequencedStore, KeyedSequencedStoreError, QueryByVector, VectorData, VectorQueryResult};

use super::VectorStoreError;

// Number of operations to run before running optimize.
const OPTIMIZE_PER_OPERATIONS: u16 = 5;

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

/// Implements a vector storage index utilizing a LanceDB instance
/// under the hood.
#[derive(Clone)]
struct LanceDBVectorStore {
    db: Connection,
    table: Table,
    table_name: String,
    schema: Arc<Schema>,
    vector_field: Arc<Field>,
    vector_len: u32,
    ops_to_optimize: u16,
}

impl LanceDBVectorStore {
    pub async fn local(data_dir: &str, table_name: String, vector_len: u32, extended_schema: Option<Schema>) -> Result<LanceDBVectorStore, LanceDBError> {
        // If this cast (usize -> i32) does not work, then there will be issues interacting with Arrow later.
        // This is an implementation detail and may change in the future.
        // Very rarely do i expect this not to work but performing the cast once here will expose the issue
        // earlier in the process of using this class.
        TryInto::<i32>::try_into(vector_len).map_err(|e| LanceDBError::InvalidParameter {
            parameter: "vector length",
            issue: "vector length could not be cast into an i32. could the number be too large for i32?",
            source: Some(e.into()),
        });

        let vector_field = build_vector_field(vector_len);
        let base_schema = build_base_schema(vector_field.clone());
        let schema = if let Some(es) = extended_schema {
            Arc::new(Schema::try_merge([base_schema, es])
                .map_err(|e| LanceDBError::InvalidParameter { 
                    parameter: "data schema or vector length", 
                    issue: "Data schema and base schema (built from vector length) could not be merged. \
                        Could there be a key conflict? Data schema must not use 'key', 'vector', or 'sequence_number' keys.",
                    source: Some(e.into()),
                })?)
        } else {
            Arc::new(base_schema)
        };

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
            vector_field,
            schema,
            vector_len,
            ops_to_optimize: OPTIMIZE_PER_OPERATIONS,
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

    pub async fn merge_insert(builder: MergeInsertBuilder, reader: impl RecordBatchReader) -> Result<(), LanceDBError> {
        merge.execute(Box::new(reader)).await
            .map_err(|e| LanceDBError::MergeInsert { source: e })?;

        Ok(())
    }
}

pub struct LanceDBEmbeddingStore<D: ArrowData, V: VectorData<D>> {
    vector_store: LanceDBVectorStore,
    _phantom_data: PhantomData<D>,
    _phantom_embedded: PhantomData<V>,
}

impl<D: ArrowData, V: VectorData<D>> LanceDBEmbeddingStore<D, V> {
    /// Construct a new LanceDBStore, given an file directory to store the data and an embedder to use to embed
    /// data before storing.
    pub async fn local(data_dir: &str, table_name: String) -> Result<LanceDBEmbeddingStore<D, V>, LanceDBError> {
        let data_schema = (*D::schema()).clone();
        let vector_len = V::vector_length();

        let vector_store = LanceDBVectorStore::local(
            data_dir,
            table_name,
            vector_len,
            Some(data_schema)
        ).await?;

        Ok(LanceDBEmbeddingStore {
            vector_store,
            _phantom_data: Default::default(),
            _phantom_embedded: Default::default(),
        })
    }
    
    // development function to clear all the data from an instantiated LanceDBStore
    pub async fn clear_all(&mut self) -> Result<(), LanceDBError> {
        self.vector_store.clear_all().await
    }
}

impl<K: Serialize, D: ArrowData, V: KeyedSequencedData<K> + VectorData<D>> KeyedSequencedStore<K, V> for LanceDBEmbeddingStore<D, V> {
    async fn put(&self, data: Vec<V>) -> Result<(), KeyedSequencedStoreError> {
        let mut row_builder = D::row_builder();
        let mut key_array = StringBuilder::new();
        let mut vector_array = FixedSizeListBuilder::new(Float32Builder::new(), 
            // unwrap okay here because theoretically, u32->i32 cast is checked earlier during constructor
            self.vector_store.vector_len.try_into().unwrap());
        let mut sequence_array = UInt64Builder::new();
        for vec_data in data {
            key_array.append_value(serde_json::to_string(&vec_data.get_key()).map_err(|e| 
                KeyedSequencedStoreError::Serialization { element: "key".to_owned(), source: e.into() })?);
            sequence_array.append_value(vec_data.get_sequence_num());
            let (d, v) = vec_data.take_data_and_vector();

            // check vector lengths at this point, before processing and then committing data
            check_vector_length(v.len() as u32, self.vector_store.vector_len)
                .map_err(|e| KeyedSequencedStoreError::Other { source: e.into() })?;

            row_builder.append(d);
            for f in v {
                vector_array.values().append_value(f);
            }
            vector_array.append(true);
        }

        let mut data_columns = vec![
            (KEY_FIELD.clone(), Arc::new(key_array.finish()) as ArrayRef)
        ];
        for field_and_array in row_builder.finish() {
            data_columns.push(field_and_array)
        }
        data_columns.push((self.vector_store.vector_field.clone(), Arc::new(vector_array.finish()) as ArrayRef));
        data_columns.push((SEQUENCE_NUMBER_FIELD.clone(), Arc::new(sequence_array.finish()) as ArrayRef));

        let struct_array = StructArray::from(data_columns);

        // push the data
        let reader = RecordBatchIterator::new(
            vec![RecordBatch::from(struct_array)]
                .into_iter()
                .map(Ok),
            self.vector_store.schema.clone(),
        );

        let mut merge = self.vector_store.table.merge_insert(&[KEY_COLUMN]);
        merge.when_matched_update_all(Some(format!("target.{SEQUENCE_NUMBER_COLUMN} < \
            source.{SEQUENCE_NUMBER_COLUMN}"))).when_not_matched_insert_all();

        merge.execute(Box::new(reader)).await
            .map_err(|e| KeyedSequencedStoreError::RecordOperation {operation: "Merge insert on record",
                source: e.into() })?;

        // TODO: optimize the table? create index for vector so it's not kNN?
        self.vector_store.table.optimize(action)

        Ok(())
    }

    async fn clear(&self, key: K, optional_sequence_number: Option<u64>) -> Result<(), KeyedSequencedStoreError> {
        let key_string = serde_json::to_string(&key).map_err(|e| 
                KeyedSequencedStoreError::Serialization { element: "key".to_owned(), source: e.into() })?;
        let mut delete_condition = format!("{KEY_COLUMN} = '{key_string}'");
        if let Some(sn) = optional_sequence_number {
            delete_condition.push_str(&format!(" AND {SEQUENCE_NUMBER_COLUMN} < {sn}"));
        }

        self.vector_store.table.delete(&delete_condition).await
            .map_err(|e| KeyedSequencedStoreError::RecordOperation { operation: "Delete record",
                source: e.into() })?;
        Ok(())
    }

    async fn get(&self, key: K) -> Result<D, KeyedSequencedStoreError> {
        todo!()
    }
}

impl<D: ArrowData, V: VectorData<D>> QueryByVector<D, V> for LanceDBEmbeddingStore<D, V> {
    fn query_vector(&self, vector: Vec<f32>) -> impl Future<Output = Result<Vec<VectorQueryResult<D>>, VectorStoreError>> {
        self.query_vector_n(vector, 0, 0)
    }

    async fn query_vector_n(&self, vector: Vec<f32>, num_results: u32, offset: u32) -> Result<Vec<VectorQueryResult<D>>, VectorStoreError> {
        check_vector_length(vector.len() as u32, V::vector_length());
        check_vector_length(self.vector_store.vector_len, V::vector_length());

        let query = self.vector_store.table.query()
            // This normally returns errors because lancedb automatically uses an embedding model if registered
            // to convert a query into a vector. However without a registered model lancedb just expects the
            // actual vector to be provided here for the query, which is what I have done. Therefore this should
            // theoretically never cause an issue.
            .nearest_to(vector).expect("Unexpected issue converting Vec<f32> to QueryVector")
            .distance_type(DistanceType::Dot)
            .column(VECTOR_COLUMN)
            // u32 -> usize casts, should always be fine
            .limit(num_results as usize)
            .offset(offset as usize);

        let mut result_stream = query.execute().await
            .map_err(|e| VectorStoreError::Query { source: e.into() })?;

        let mut result_list: Vec<VectorQueryResult<D>> = Vec::new();
        while let Some(rb) = result_stream.next().await {
            match rb {
                Ok(batch) => {
                    let vector_column = batch.column_by_name(VECTOR_COLUMN)
                        .expect("vector column should exist in vector query")
                        .as_any().downcast_ref::<FixedSizeListArray>()
                        .expect("Returned query result of vectors could not be cast to an array")
                        .iter()
                        .map(|a| a.expect("vector should exist")
                            .as_primitive::<Float32Type>()
                            .values())
                        // from(&[f32]->Vec)
                        // into(&ScalarBuffer->&[f32])
                        .map(|b| Vec::from(b.into()))
                        .collect::<Vec<Vec<f32>>>();
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

                    let vector_iter = vector_column.into_iter();
                    let data_iter = D::batch_to_iter(batch).into_iter();
                    let distance_iter = distance_column.into_iter();

                    while let (
                        Some(data),
                        Some(vector),
                        Some(distance)
                    ) = (
                        data_iter.next(),
                        vector_iter.next(),
                        distance_iter.next()
                    ) {
                        result_list.push(VectorQueryResult {
                            result: data,
                            vector,
                            distance,
                        })
                    }
                    if data_iter.next().is_some() || vector_iter.next().is_some() || distance_iter.next().is_some() {
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

pub mod integrations;

// Private variables and methods

const TABLE_NAME: &str = "vector_index";
const KEY_COLUMN: &str = "key";
const VECTOR_COLUMN: &str = "vector";
const SEQUENCE_NUMBER_COLUMN: &str = "sequence_number";

static KEY_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(KEY_COLUMN, DataType::Utf8, false))
});

static SEQUENCE_NUMBER_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(SEQUENCE_NUMBER_COLUMN, DataType::UInt64, false))
});

fn build_vector_field(vector_len: u32) -> Arc<Field> {
    Arc::new(Field::new(
        VECTOR_COLUMN,
        // have not been able to make this work as non-nullable. no matter what I put when inserting records,
        // lancedb somehow always ends up assuming the records are nullable while the schema is not.
        // if I then change this fixed size list data type to accept null but dont also change the produced records,
        // suddenly the records are not nullable anymore.
        //
        // vector_len.try_into() --> u32 -> i32 conversion. will most likely work and will error if absurd vector_
        // len provided (> signed int max)
        DataType::new_fixed_size_list(DataType::Float32, vector_len.try_into().unwrap(), true), // casting to i32
        false,
    ));
}

/// Builds a base schema object given a number of floats that the embedded vector will occupy
/// This schema object should be merged with the data schema to make the full schema
fn build_base_schema(vector_field: Arc<Field>) -> Schema {
    Schema::new(vec![
        KEY_FIELD,
        vector_field,
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