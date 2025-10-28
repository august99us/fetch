use std::future::Future;

use crate::{indexing::Indexable, vector_store::{IndexVector, QueryVectorKeys}};

use super::FileIndexer;

/// Describes an object that understands how to perform semantic queries against indexed files.
/// 
/// This trait provides methods for finding files that are semantically similar to a given
/// text description by converting the description to an embedding and querying the index by
/// similarity to that embedding. Both methods support pagination to handle large result sets.
pub trait QueryFiles {
    /// Query for files matching description provided, returning a default number of results (currently 20).
    /// 
    /// # Arguments
    /// * `file_description` - The text description to search for
    /// * `page` - Optional page number (1-based). If None, defaults to page 1
    /// 
    /// # Returns
    /// Returns the results for the specified page, with each page containing up to 20 results.
    async fn query(&self, file_description: &str, page: Option<u32>) -> Result<FileQuerying::Result, FileQuerying::Error>;
    
    /// Query for files matching description provided, returning a given number of results per page.
    /// 
    /// # Arguments
    /// * `file_description` - The text description to search for
    /// * `num_results` - Number of results to return per page
    /// * `page` - Page number (1-based). Page 1 returns results 1-num_results, page 2 returns
    ///   results (num_results+1)-(2*num_results), etc.
    /// 
    /// # Returns
    /// Returns the results for the specified page with the specified number of results per page.
    async fn query_n(&self, file_description: &str, num_results: u32, page: u32) -> Result<FileQuerying::Result, FileQuerying::Error>;
}

impl<I: IndexVector + QueryVectorKeys + Send + Sync> QueryFiles for FileIndexer<I> {
    // Query 20 results by default, starting from page 1 if no page specified
    fn query(&self, file_description: &str, page: Option<u32>) -> impl Future<Output = Result<FileQuerying::Result, FileQuerying::Error>> {
        self.query_n(file_description, 20, page.unwrap_or(1))
    }

    async fn query_n(&self, file_description: &str, num_results: u32, page: u32) -> Result<FileQuerying::Result, FileQuerying::Error> {
        if page == 0 {
            return Err(FileQuerying::Error { query: file_description.to_string(), 
                source: anyhow::Error::msg("Page number must be 1 or greater"), r#type: FileQuerying::ErrorType::Query });
        }

        let query_vector = file_description.calculate_embedding().await.map_err(|e| 
            FileQuerying::Error { query: file_description.to_string(), source: e.into(), r#type: FileQuerying::ErrorType::Embedding })?;

        // Calculate offset for pagination (page 1 = offset 0, page 2 = offset num_results, etc.)
        let offset = (page - 1) * num_results;
        match self.vector_store.query_n_keys(query_vector, num_results, offset).await {
            Ok(list) => Ok(FileQuerying::Result::from(list)),
            Err(e) => Err(FileQuerying::Error { query: file_description.to_string(), 
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