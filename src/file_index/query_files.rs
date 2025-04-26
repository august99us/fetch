use std::future::Future;

use crate::{embeddable::Embeddable, vector_store::{IndexVector, QueryVectorKeys}};

use super::FileIndexer;

pub trait QueryFiles {
    async fn query(&self, file_description: &str) -> Result<FileQuerying::Result, FileQuerying::Error>;
    async fn query_n(&self, file_description: &str, num_results: u32) -> Result<FileQuerying::Result, FileQuerying::Error>;
}

impl<I: IndexVector + QueryVectorKeys + Send + Sync> QueryFiles for FileIndexer<I> {
    // Query 15 by default
    fn query(&self, file_description: &str) -> impl Future<Output = Result<FileQuerying::Result, FileQuerying::Error>> {
        self.query_n(file_description, 15)
    }

    async fn query_n(&self, file_description: &str, num_results: u32) -> Result<FileQuerying::Result, FileQuerying::Error> {
        let query_vector = file_description.calculate_embedding().await.map_err(|e| 
            FileQuerying::Error { query: file_description.to_string(), source: e.into(), r#type: FileQuerying::ErrorType::Embedding })?;

        match self.vector_store.query_n_keys(query_vector, num_results).await {
            Ok(list) => Ok(FileQuerying::Result::from(list)),
            Err(e) => Err(FileQuerying::Error {query: file_description.to_string(), 
                source: e.into(), r#type: FileQuerying::ErrorType::Query }),
        }
    }
}

pub mod FileQuerying {
    pub use super::result::FileQueryingResult as Result;
    pub use super::error::{FileQueryingError as Error, FileQueryingErrorType as ErrorType};
}
mod result;
mod error;