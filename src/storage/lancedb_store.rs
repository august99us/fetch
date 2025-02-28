use std::{future::Future, io::Read, sync::{Arc, OnceLock}, time::SystemTime};

use arrow::datatypes::Float32Type;
use arrow_array::{FixedSizeListArray, RecordBatch, RecordBatchIterator, StringArray, TimestampMillisecondArray};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use futures::stream::StreamExt;
use lancedb::{connect, database::CreateTableMode, query::{ExecutableQuery, QueryBase, Select}, Error, Table};

use crate::{embeddable::Embeddable, Preview};

use super::{IndexPreview, QuerySimilarFiles};

pub struct LanceDBStore {
    table: Table,
}

impl LanceDBStore {
    pub async fn new(data_dir: &str) -> Result<LanceDBStore, Error> {
        // randomly chosen number, need some other method of determining this, either through some constant or some model
        // result
        let db = connect(data_dir).execute().await?;
        let table = db.create_empty_table("semantic_preview_index", schema())
            .mode(CreateTableMode::ExistOk(Box::new(|r| r)))
            .execute().await?;
        Ok(LanceDBStore {
            table,
        })
    }
}

impl IndexPreview for LanceDBStore {
    // TODO: clean up error handling
    async fn index<R: Read>(&self, preview: Preview<R>) -> Result<(), String> {
        let embedding = preview.calculate_embedding()?;
        //let bytes = preview.content.collect::<Result<Vec<u8>, std::io::Error>>().map_err(|e| e.to_string())?;
        let batches = RecordBatchIterator::new(
            vec![RecordBatch::try_new(
                schema(),
                vec![
                    Arc::new(StringArray::from_iter_values(vec![preview.path])),
                    Arc::new(FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        vec![Some(embedding.map(|v| Some(v)))],
                        VECTOR_LEN,
                    )),
                    Arc::new(TimestampMillisecondArray::from_iter_values(vec![i64::try_from(preview.timestamp.duration_since(SystemTime::UNIX_EPOCH).expect("time before epoch").as_millis()).expect("millis since unix epoch over i64 max")]))
                ],
            )
            .unwrap()]
            .into_iter()
            .map(Ok),
            schema(),
        );

        let mut merge = self.table.merge_insert(&["path"]);
        merge.when_matched_update_all(Some(String::from("target.timestamp < source.timestamp")))
            .when_not_matched_insert_all();

        merge.execute(Box::new(batches)).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<(), String> {
        self.table.delete(&format!("path = {}", path)).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

impl QuerySimilarFiles for LanceDBStore {
     fn query(&self, file_description: &str) -> impl Future<Output = Result<Vec<String>, String>> {
        self.query_n(file_description, 15)
    }

    async fn query_n(&self, file_description: &str, num_files: usize) -> Result<Vec<String>, String> {
        let embedding = file_description.calculate_embedding()?;
        let query = self.table.query()
            .nearest_to(&embedding).map_err(|e| e.to_string())?
            .column("embedding")
            .select(Select::Columns(vec![String::from("path")]))
            .limit(num_files);
        let mut result_stream = query.execute().await.map_err(|e| e.to_string())?;
        let mut result_vec: Vec<String> = Vec::new();
        while let Some(rb) = result_stream.next().await {
            // TODO: theres some intense error handling to be done here........
            result_vec.extend(rb.map_err(|e| e.to_string())?
                .column(0) // Pick out the only column in the query, the "path" column
                .as_any().downcast_ref::<StringArray>().unwrap().iter() // cast it to string array and get iterator
                .map(|s| s.unwrap().to_string()) // unwrap the optionals
                .collect::<Vec<String>>() // collect into a vector
            ); // extend result_vec with the values in the collected string vector
        }
        Ok(result_vec)
    }
}

// Private functions

const VECTOR_LEN: i32 = 1000;

fn schema() -> Arc<Schema> {
    static SCHEMA: OnceLock<Arc<Schema>> = OnceLock::new();
    SCHEMA.get_or_init(|| Arc::new(Schema::new(vec![
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
            DataType::new_fixed_size_list(DataType::Float32, VECTOR_LEN, false),
            false,
        ),
        Field::new(
            "timestamp",
            DataType::Timestamp(TimeUnit::Millisecond, None),
            false,
        ),
    ]))).clone()
}