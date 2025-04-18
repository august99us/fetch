use std::future::Future;

use crate::{embeddable::Embeddable, vector_store::{IndexVector, QueryVectorKeys}};

use super::FileIndexer;

pub trait QueryFiles {
    async fn query(&self, file_description: &str) -> Result<Vec<String>, FileQuerying::Error>;
    async fn query_n(&self, file_description: &str, num_results: u32) -> Result<Vec<String>, FileQuerying::Error>;
}

impl<I: IndexVector + QueryVectorKeys> QueryFiles for FileIndexer<I> {
    // Query 15 by default
    fn query(&self, file_description: &str) -> impl Future<Output = Result<Vec<String>, FileQuerying::Error>> {
        self.query_n(file_description, 15)
    }

    async fn query_n(&self, file_description: &str, num_results: u32) -> Result<Vec<String>, FileQuerying::Error> {
        let query_vector = file_description.calculate_embedding(&self.embedder).await.map_err(|e| 
            FileQuerying::Error { query: file_description.to_string(), source: e, r#type: FileQuerying::ErrorType::Embedding })?;

        match self.vector_store.query_n_keys(query_vector, num_results).await {
            Ok(list) => Ok(list.into_iter().map(|r| r.key).collect()),
            Err(e) => Err(FileQuerying::Error {query: file_description.to_string(), 
                source: Box::new(e), r#type: FileQuerying::ErrorType::Query }),
        }
    }
}

pub mod FileQuerying {
    pub use super::error::{FileQueryingError as Error, FileQueryingErrorType as ErrorType};
}
mod error;