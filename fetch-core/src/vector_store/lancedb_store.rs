use std::{future::Future, sync::Arc};

use arrow_array::{Array, FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator, StringArray, UInt64Array};
use arrow_schema::{DataType, Field, Schema};
use futures::stream::StreamExt;
use lancedb::{connect, database::CreateTableMode, query::{ExecutableQuery, QueryBase, Select}, Connection, DistanceType, Table};

use super::{IndexVector, QueryKeyResult, QueryVectorKeys, VectorStoreError};

/// Implements a preview storage index utilizing a local file system LanceDB instance
/// under the hood.
/// Provides the IndexPreview and QuerySimilarFiles traits through that LanceDB backend.
/// 
/// Instantiate this 

const TABLE_NAME: &str = "vector_index";
const KEY_COLUMN: &str = "key";
const VECTOR_COLUMN: &str = "vector";
const SEQUENCE_NUMBER_COLUMN: &str = "sequence_number";

#[derive(Clone)]
pub struct LanceDBStore {
    db: Connection,
    table: Table,
    schema: Arc<Schema>,
    vector_len: u32,
}

#[derive(thiserror::Error, Debug)]
pub enum LanceDBError {
    #[error("Invalid parameters provided to constructor")]
    InvalidParameter { parameter: &'static str, issue: &'static str },
    #[error("Issue creating connection to data directory")]
    Connection (#[source] lancedb::error::Error),
    #[error("Error performing holistic table operations")]
    TableOperation { operation: &'static str, #[source] source: lancedb::error::Error },
}

impl LanceDBStore {
    /// Construct a new LanceDBStore, given an file directory to store the data and an embedder to use to embed
    /// data before storing.
    /// 
    /// Embedder must be of variant VisionEmbedder because only embedders of those variants
    /// satisfy both the EmbedImage and EmbedText traits. Will return an InvalidParameterError if a text embedder
    /// is provided.
    /// 
    /// Ideally the result vector length is an inherent element of the embedder passed in arguments and doesn't 
    /// need its own argument. Perhaps this can be updated later. If the vector length does not match the vector
    /// length in the previously created table a LanceDBError::TableOperationError will be returned.
    pub async fn new(data_dir: &str, vector_len: u32) -> Result<LanceDBStore, LanceDBError> {
        // If this cast (usize -> i32) does not work, then there will be issues interacting with Arrow later.
        // This is an implementation detail and may change in the future.
        // Very rarely do i expect this not to work but performing the cast once here will expose the issue
        // earlier in the process of using this class.
        TryInto::<i32>::try_into(vector_len).unwrap();

        let db = connect(data_dir)
            .execute().await
            .map_err(LanceDBError::Connection)?;
        let schema = build_schema(vector_len);
        let table = db.create_empty_table(TABLE_NAME, schema.clone())
            .mode(CreateTableMode::ExistOk(Box::new(|r| r)))
            .execute().await
            .map_err(|e| LanceDBError::TableOperation { operation: "Creating or opening table", source: e })?;

        Ok(LanceDBStore {
            db,
            table,
            schema,
            vector_len,
        })
    }

    // development function to clear all the data from a given directory with LanceDB data inside
    pub async fn drop(data_dir: &str) -> Result<(), LanceDBError> {
        let db = connect(data_dir)
            .execute().await
            .map_err(LanceDBError::Connection)?;
        db.drop_all_tables().await
            .map_err(|e| LanceDBError::TableOperation { operation: "Dropping all tables", source: e })?;
        Ok(())
    }
    
    // development function to clear all the data from an instantiated LanceDBStore
    pub async fn clear(&mut self) -> Result<(), LanceDBError> {
        self.db.drop_all_tables().await
            .map_err(|e| LanceDBError::TableOperation { operation: "Dropping all tables", source: e })?;
        let schema = build_schema(self.vector_len);
        self.table = self.db.create_empty_table(TABLE_NAME, schema.clone())
            .mode(CreateTableMode::ExistOk(Box::new(|r| r)))
            .execute().await
            .map_err(|e| LanceDBError::TableOperation { operation: "Creating or opening table", source: e })?;
        Ok(())
    }

    /// Return a new copy of the Schema pointer
    pub fn schema(&self) -> Arc<Schema> {
        self.schema.clone()
    }
}

impl IndexVector for LanceDBStore {
    async fn index(&self, key: &str, vector: Vec<f32>, sequence_number: u64) -> Result<(), VectorStoreError> {
        verify_valid_vector_len(self, &vector)?;

        let batches = RecordBatchIterator::new(
            vec![RecordBatch::try_new(
                self.schema(),
                vec![
                    // hash key column
                    Arc::new(StringArray::from_iter_values(vec![key])),
                    // --------------------
                    // value column (the vector calculated from embedding)
                    Arc::new(
                        FixedSizeListArray::try_new(
                            Arc::new(Field::new_list_field(DataType::Float32, true)),
                            self.vector_len.try_into().unwrap(), // casting into i32
                            Arc::new(Float32Array::from(vector)),
                            None
                        // Expect: the embedding vector should convert without error because the length of the
                        // embedding vector is hard coded into the program as of now.
                        ).unwrap_or_else(|e| panic!("Error creating FixedSizeListArray from embedding for key {:?}\n\
                            Error was: {:?}", key.to_string(), e))
                    ),
                    // --------------------
                    // sequence number column
                    Arc::new(UInt64Array::from_iter_values(vec![sequence_number]))
                ],
            )
            // Expect: panic here because there is no real processing other than reorganization of information
            // happening on the elements here, unsure how code would react to an Error provided by this section
            .expect("Issue creating RecordBatch from inert elements")]
            .into_iter()
            .map(Ok),
            self.schema(),
        );

        let mut merge = self.table.merge_insert(&[KEY_COLUMN]);
        merge.when_matched_update_all(Some(format!("target.{SEQUENCE_NUMBER_COLUMN} < source.{SEQUENCE_NUMBER_COLUMN}"))).when_not_matched_insert_all();

        merge.execute(Box::new(batches)).await
            .map_err(|e| VectorStoreError::RecordOperation { record_key: key.to_string(), 
                operation: "Merge insert on record", source: e.into() })?;

        Ok(())
    }

    async fn delete(&self, key: &str, optional_sequence_number: Option<u64>) -> Result<(), VectorStoreError> {
        let mut delete_condition = format!("{KEY_COLUMN} = '{key}'");
        if let Some(sn) = optional_sequence_number {
            delete_condition.push_str(&format!(" AND {SEQUENCE_NUMBER_COLUMN} < {sn}"));
        }

        self.table.delete(&delete_condition).await
            .map_err(|e| VectorStoreError::RecordOperation { record_key: key.to_string(),
                operation: "Delete record", source: e.into() })?;
        Ok(())
    }
}

impl QueryVectorKeys for LanceDBStore {
    // Query all results by default
    fn query_keys(&self, vector: Vec<f32>) -> impl Future<Output = Result<Vec<QueryKeyResult>, VectorStoreError>> {
        self.query_n_keys(vector, 0, 0)
    }

    async fn query_n_keys(&self, vector: Vec<f32>, num_results: u32, offset: u32) -> Result<Vec<QueryKeyResult>, VectorStoreError> {
        verify_valid_vector_len(self, &vector)?;

        let mut query = self.table.query()
            // This normally returns errors because lancedb automatically uses an embedding model if registered
            // to convert a query into a vector. However without a registered model lancedb just expects the
            // actual vector to be provided here for the query, which is what I have done. Therefore this should
            // theoretically never cause an issue.
            .nearest_to(vector).expect("Unexpected issue converting Vec<f32> to QueryVector")
            .distance_type(DistanceType::Dot)
            .column(VECTOR_COLUMN)
            .select(Select::Columns(vec![String::from(KEY_COLUMN)]))
            .offset(offset as usize);

        if num_results > 0 {
            // u32 -> usize cast, should always be fine
            query = query.limit(num_results.try_into().unwrap());
        }
        let mut result_stream = query.execute().await
            .map_err(|e| VectorStoreError::Query { source: e.into() })?;

        let mut result_list: Vec<QueryKeyResult> = Vec::new();
        while let Some(rb) = result_stream.next().await {
            match rb {
                Ok(batch) => {
                    let string_column = batch.column_by_name(KEY_COLUMN) // Pick out the key column
                        .expect("key column should exist in vector query")
                        // cast to a string array
                        .as_any().downcast_ref::<StringArray>()
                        // Expect: the column is defined as a string in the schema, this conversion from arrow array
                        // should not fail.
                        .expect("Returned query result of keys could not be cast to a string")
                        // unwrap the optionals
                        // Expect: the column is defined as non-nullable in the schema, there should be no reason for
                        // the optional to be empty
                        .iter().map(|s| s.expect("Missing string in optional for a non-nullable key column")
                            .to_string())
                        .collect::<Vec<String>>();
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

                    for (i, key) in string_column.into_iter().enumerate() {
                        // Unwrap should work because both columns are required for all rows (square array)
                        result_list.push(QueryKeyResult { key, distance: *distance_column.get(i).unwrap() });
                    }
                }
                Err(e) => return Err(VectorStoreError::Query { source: e.into() })
            }
        }
        Ok(result_list)
    }
}

// Private functions

/// Common code to verify that a vector argument has the correct lenght to interact with vectors
/// stored in a particular instance of LanceDBStore
fn verify_valid_vector_len(store: &LanceDBStore, vector: &Vec<f32>) -> Result<(), VectorStoreError> {
    // converting u32 -> usize should always work
    if vector.len() != store.vector_len as usize {
        // converting usize -> u32, should always be functional unless an absurdly sized vector is provided
        return Err(VectorStoreError::InvalidVectorLength { inputted_vector_len: vector.len().try_into().unwrap(),
            required_vector_len: store.vector_len });
    }
    Ok(())
}

/// Builds a schema object given a number of floats that the embedded vector will occupy
fn build_schema(vector_len: u32) -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        // Do i need an ID field? would require a path-> id index. would defend against changes in data model.
        // dropping table is probably not too expensive considering this application is only meant to be used
        // for personal purposes
        Field::new(
            KEY_COLUMN,
            DataType::Utf8,
            false,
        ),
        Field::new(
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
        ),
        Field::new(
            SEQUENCE_NUMBER_COLUMN,
            DataType::UInt64,
            false,
        ),
    ]))
}