use std::{error::Error, future::Future, io::Read, sync::{Arc, OnceLock}, time::SystemTime};

use arrow_array::{Array, FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator, StringArray, TimestampMillisecondArray};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use futures::stream::StreamExt;
use lancedb::{connect, database::CreateTableMode, query::{ExecutableQuery, QueryBase, Select}, Connection, Table};

use crate::{embeddable::Embeddable, Preview};

use super::{IndexPreview, QuerySimilarFiles};

pub struct LanceDBStore {
    db: Connection,
    table: Table,
}

impl LanceDBStore {
    pub async fn new(data_dir: &str) -> Result<LanceDBStore, Box<dyn Error>> {
        // randomly chosen number, need some other method of determining this, either through some constant or some model
        // result
        let db = connect(data_dir).execute().await?;
        let table = db.create_empty_table("semantic_preview_index", schema())
            .mode(CreateTableMode::ExistOk(Box::new(|r| r)))
            .execute().await?;
        Ok(LanceDBStore {
            db,
            table,
        })
    }

    pub async fn clear(&mut self) -> Result<(), Box<dyn Error>> {
        self.db.drop_all_tables().await?;
        self.table = self.db.create_empty_table("semantic_preview_index", schema())
            .mode(CreateTableMode::ExistOk(Box::new(|r| r)))
            .execute().await?;
        Ok(())
    }
}

impl IndexPreview for LanceDBStore {
    // TODO: clean up error handling
    async fn index<R: Read>(&self, preview: Preview<R>) -> Result<(), Box<dyn Error>> {
        let embedding = preview.calculate_embedding()?;
        println!("embedding is this: {:?}", embedding);
        let wrapped_values = Float32Array::from_iter_values(embedding);
        let batches = RecordBatchIterator::new(
            vec![RecordBatch::try_new(
                schema(),
                vec![
                    Arc::new(StringArray::from_iter_values(vec![preview.path])),
                    Arc::new(FixedSizeListArray::try_new(
                        Arc::new(Field::new_list_field(DataType::Float32, true)),
                        VECTOR_LEN,
                        Arc::new(wrapped_values),
                        None
                    )?),
                    /* 
                    Arc::new(FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        vec![embedding],
                        VECTOR_LEN,
                    )),
                    */
                    Arc::new(TimestampMillisecondArray::from_iter_values(vec![i64::try_from(preview.timestamp
                        .duration_since(SystemTime::UNIX_EPOCH).expect("time before epoch").as_millis())
                        .expect("millis since unix epoch over i64 max")]))
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

        merge.execute(Box::new(batches)).await?;
        println!("testing a lot of other things checkpoint 2");

        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<(), Box<dyn Error>> {
        self.table.delete(&format!("path = {}", path)).await?;
        Ok(())
    }
}

impl QuerySimilarFiles for LanceDBStore {
    fn query(&self, file_description: &str) -> impl Future<Output = Result<Vec<String>, Box<dyn Error>>> {
        self.query_n(file_description, 15)
    }

    async fn query_n(&self, file_description: &str, num_files: usize) -> Result<Vec<String>, Box<dyn Error>> {
        let embedding = file_description.calculate_embedding()?;
        println!("something something something {:?}", embedding);
        let query = self.table.query()
            .nearest_to(&embedding)?
            .column("embedding")
            .select(Select::Columns(vec![String::from("path")]))
            .limit(num_files);
        let mut result_stream = query.execute().await?;
        let mut result_vec: Vec<String> = Vec::new();
        while let Some(rb) = result_stream.next().await {
            // TODO: theres some intense error handling to be done here........
            result_vec.extend(rb?.column(0) // Pick out the only column in the query, the "path" column
                .as_any().downcast_ref::<StringArray>().unwrap().iter() // cast it to string array and get iterator
                .map(|s| s.unwrap().to_string()) // unwrap the optionals
                .collect::<Vec<String>>() // collect into a vector
            ); // extend result_vec with the values in the collected string vector
        }
        Ok(result_vec)
    }
}

// Private functions

const VECTOR_LEN: i32 = 512;

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
            // have not been able to make this work as non-nullable. no matter what I put when inserting records,
            // lancedb somehow always ends up assuming the records are nullable while the schema is not.
            // if I then change this fixed size list data type to accept null but dont also change the produced records,
            // suddenly the records are not nullable anymore.
            DataType::new_fixed_size_list(DataType::Float32, VECTOR_LEN, true),
            false,
        ),
        Field::new(
            "timestamp",
            DataType::Timestamp(TimeUnit::Millisecond, None),
            false,
        ),
    ]))).clone()
}