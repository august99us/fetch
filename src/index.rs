use std::{error::Error, io::Read};

use crate::Preview;

pub enum StorageError {

}

/// index
pub trait IndexPreview {
    async fn index<R: Read>(&self, preview: Preview<R>) -> Result<(), Box<dyn Error>>;
    async fn delete(&self, path: &str) -> Result<(), Box<dyn Error>>;
}

/// query and return path
pub trait QuerySimilarFiles {
    async fn query(&self, file_description: &str) -> Result<Vec<String>, Box<dyn Error>>;
    async fn query_n(&self, file_description: &str, num_files: usize) -> Result<Vec<String>, Box<dyn Error>>;
}

pub mod lancedb_store;