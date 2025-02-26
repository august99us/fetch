use std::collections::HashMap;

use arrow_schema::{DataType, Field, Schema};
use lancedb::{connect, embeddings::EmbeddingFunction, Connection};

use crate::previewable::PreviewType;

use super::IndexPreview;

pub struct LanceDBStore {
    db: Connection,
    schema: Schema,
}

impl LanceDBStore {
    pub fn new(data_dir: &str, embedding_functions: HashMap<PreviewType, EmbeddingFunction>) -> LanceDBStore {
        LanceDBStore {
            db: connect(data_dir),
            schema: Schema::new(vec![
                Field::new(
                    "id",
                    DataType::Utf8,
                    true,
                ),
                Field::new(
                    "path",
                    DataType::Utf8,
                    true,
                ),
                Field::new(
                    "text",
                    DataType::Bytes,
                    false,
                ),
                Field::new(
                    "image",
                    DataType::Bytes,
                    false,
                ),
            ]),
        }
    }
    
    pub async fn execute(&self) -> LanceDBStore {
        self.db.execute().await?;
        self
    }
}

impl IndexPreview for LanceDBStore {

}

impl QueryPreview for LanceDBStore {

}