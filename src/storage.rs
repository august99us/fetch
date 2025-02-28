use std::io::Read;

use crate::Preview;

// TODO REPLACE ERROR TYPES
/// index
pub trait IndexPreview {
    async fn index<R: Read>(&self, preview: Preview<R>) -> Result<(), String>;
    async fn delete(&self, path: &str) -> Result<(), String>;
}

/// query and return path
pub trait QuerySimilarFiles {
    async fn query(&self, file_description: &str) -> Result<Vec<String>, String>;
    async fn query_n(&self, file_description: &str, num_files: usize) -> Result<Vec<String>, String>;
}

pub mod lancedb_store;