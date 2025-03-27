use std::{future::Future, sync::Arc, time::SystemTime};

use arrow_array::{Array, FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator, StringArray, TimestampMillisecondArray};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use embed_anything::embeddings::embed::Embedder;
use futures::stream::StreamExt;
use lancedb::{connect, database::CreateTableMode, query::{ExecutableQuery, QueryBase, Select}, Connection, Table};

use crate::{embeddable::Embeddable, Preview};

use super::{IndexPreview, QuerySimilarFiles, SemanticIndexError};

/// Implements a preview storage index utilizing a local file system LanceDB instance
/// under the hood.
/// Provides the IndexPreview and QuerySimilarFiles traits through that LanceDB backend.
/// 
/// Instantiate this 
pub struct LanceDBStore {
    db: Connection,
    table: Table,
    schema: Arc<Schema>,
    embedder: Embedder,
    vector_len: i32,
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
    pub async fn new(data_dir: &str, embedder: Embedder, embedding_vector_len: i32) -> Result<LanceDBStore, LanceDBError> {
        if let Embedder::Text(_) = embedder {
            return Err(LanceDBError::InvalidParameter { parameter: "Embedder", 
                issue: "Was text embedder" });
        }

        let db = connect(data_dir)
            .execute().await
            .map_err(|e| LanceDBError::Connection(e))?;
        let schema = build_schema(embedding_vector_len);
        let table = db.create_empty_table("semantic_preview_index", schema.clone())
            .mode(CreateTableMode::ExistOk(Box::new(|r| r)))
            .execute().await
            .map_err(|e| LanceDBError::TableOperation { operation: "Creating or opening table", source: e })?;

        Ok(LanceDBStore {
            db,
            table,
            schema,
            embedder,
            vector_len: embedding_vector_len,
        })
    }

    /// Return a new copy of the Schema pointer
    pub fn schema(&self) -> Arc<Schema> {
        self.schema.clone()
    }

    // development function to clear all the data from the semantic preview index table.
    pub async fn clear(&mut self) -> Result<(), LanceDBError> {
        self.db.drop_all_tables().await
            .map_err(|e| LanceDBError::TableOperation { operation: "Dropping all tables", source: e })?;
        self.table = self.db.create_empty_table("semantic_preview_index", self.schema())
            .mode(CreateTableMode::ExistOk(Box::new(|r| r)))
            .execute().await
            .map_err(|e| LanceDBError::TableOperation { operation: "Creating table", source: e })?;
        Ok(())
    }
}

impl IndexPreview for LanceDBStore {
    async fn index<'a>(&self, preview: Preview<'a>) -> Result<(), SemanticIndexError> {
        let embedding = preview.calculate_embedding(&self.embedder).await
            .map_err(|e| SemanticIndexError::PreviewEmbedding { record_key: preview.original_file_path.to_string(), source: e })?;
        let batches = RecordBatchIterator::new(
            vec![RecordBatch::try_new(
                self.schema(),
                vec![
                    // hash key column
                    Arc::new(StringArray::from_iter_values(vec![&preview.original_file_path])),
                    // --------------------
                    // value column (the vector calculated from embedding)
                    Arc::new(
                        FixedSizeListArray::try_new(
                            Arc::new(Field::new_list_field(DataType::Float32, true)),
                            self.vector_len,
                            Arc::new(Float32Array::from(embedding)),
                            None
                        // Expect: the embedding vector should convert without error because the length of the
                        // embedding vector is hard coded into the program as of now.
                        ).unwrap_or_else(|e| panic!("Error creating FixedSizeListArray from embedding for file \
                            path {:?}\nError was: {:?}", preview.path.to_string(), e))
                    ),
                    // --------------------
                    // timestamp column
                    Arc::new(TimestampMillisecondArray::from_iter_values(vec![i64::try_from(preview.timestamp
                        .duration_since(SystemTime::UNIX_EPOCH).expect("time before epoch").as_millis())
                        // not expected to happen for millions of years
                        .expect("millis since unix epoch over i64 max")]))
                ],
            )
            // Expect: panic here because there is no real processing other than reorganization of information
            // happening on the elements here, unsure how code would react to an Error provided by this section
            .expect("Issue creating RecordBatch from inert elements")]
            .into_iter()
            .map(Ok),
            self.schema(),
        );

        let mut merge = self.table.merge_insert(&["path"]);
        merge.when_matched_update_all(Some(String::from("target.timestamp < source.timestamp")))
            .when_not_matched_insert_all();

        merge.execute(Box::new(batches)).await
            .map_err(|e| SemanticIndexError::RecordOperation { record_key: preview.path.to_string(), 
                operation: "Merge insert on record", source: Box::new(e) })?;

        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<(), SemanticIndexError> {
        self.table.delete(&format!("path = {}", path)).await
            .map_err(|e| SemanticIndexError::RecordOperation { record_key: path.to_string(),
                operation: "Delete record", source: Box::new(e) })?;
        Ok(())
    }
}

impl QuerySimilarFiles for LanceDBStore {
    fn query(&self, file_description: &str) -> impl Future<Output = Result<Vec<String>, SemanticIndexError>> {
        self.query_n(file_description, 15)
    }

    async fn query_n(&self, file_description: &str, num_files: usize) -> Result<Vec<String>, SemanticIndexError> {
        let embedding = file_description.calculate_embedding(&self.embedder).await
            .map_err(|e| SemanticIndexError::QueryEmbedding { query: file_description.to_owned(), source: e })?;
        let query = self.table.query()
            // This normally returns errors because lancedb automatically uses an embedding model if registered
            // to convert a query into a vector. However without a registered model lancedb just expects the
            // actual vector to be provided here for the query, which is what I have done. Therefore this should
            // theoretically never cause an issue.
            .nearest_to(embedding).expect("Unexpected issue converting Vec<f32> to QueryVector")
            .column("embedding")
            .select(Select::Columns(vec![String::from("path")]))
            .limit(num_files);
        let mut result_stream = query.execute().await
            .map_err(|e| SemanticIndexError::Query { query: file_description.to_string(), source: Box::new(e) })?;

        let mut result_vec: Vec<String> = Vec::new();
        while let Some(rb) = result_stream.next().await {
            match rb {
                Ok(batch) => {
                    result_vec.extend(batch.column(0) // Pick out the only column in the query, the "path" column
                        // cast to a string array
                        // Expect: the column is defined as a string in the schema, this conversion from arrow array
                        // should not fail.
                        .as_any().downcast_ref::<StringArray>()
                        .expect("Returned query result of file paths could not be cast to a string")
                        // unwrap the optionals
                        // Expect: the column is defined as non-nullable in the schema, there should be no reason for
                        // the optional to be empty
                        .iter().map(|s| s.expect("Missing string in optional for a non-nullable column")
                            .to_string())
                        .collect::<Vec<String>>()); // collect into a vector and extend result_vec with values
                }
                Err(e) => return Err(SemanticIndexError::Query { query: file_description.to_string(), source: Box::new(e) })
            }
        }
        Ok(result_vec)
    }
}

// Private functions

// Builds a schema object given a number of floats that the embedded vector will occupy
fn build_schema(embedding_vector_len: i32) -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        // Do i need an ID field? would require a path-> id index. would defend against changes in data model.
        // dropping table is probably not too expensive considering this application is only meant to be used
        // for personal purposes
        Field::new(
            "path",
            DataType::Utf8,
            false,
        ),
        Field::new(
            "embedding",
            // have not been able to make this work as non-nullable. no matter what I put when inserting records,
            // lancedb somehow always ends up assuming the records are nullable while the schema is not.
            // if I then change this fixed size list data type to accept null but dont also change the produced records,
            // suddenly the records are not nullable anymore.
            DataType::new_fixed_size_list(DataType::Float32, embedding_vector_len, true),
            false,
        ),
        Field::new(
            "timestamp",
            DataType::Timestamp(TimeUnit::Millisecond, None),
            false,
        ),
    ]))
}