use std::time::SystemTime;

use camino::Utf8Path;
use futures::future::join_all;

use crate::{embeddable::Embeddable, previewable::{PossiblyPreviewable, PreviewError, PreviewedFile}, vector_store::{IndexVector, QueryVectorKeys}};

use super::FileIndexer;

/// index
pub trait IndexFiles {
    /// Index a file, first attempting to load it, then generating a preview for the file, then embedding it, and
    /// finally indexing it into a vector store
    async fn index<'a>(&self, path: &'a Utf8Path) -> Result<FileIndexing::Result<'a>, FileIndexing::Error>;
    async fn index_multiple<'a>(&self, paths: Vec<&'a Utf8Path>) -> Vec<Result<FileIndexing::Result<'a>, FileIndexing::Error>>;
    /// Clear the index for a file path. Does not check for the existence of the file
    async fn clear<'a>(&self, path: &'a Utf8Path) -> Result<FileIndexing::Result<'a>, FileIndexing::Error>;
}

impl<I: IndexVector + QueryVectorKeys> IndexFiles for FileIndexer<I> {
    async fn index<'a>(&self, path: &'a Utf8Path) -> Result<FileIndexing::Result<'a>, FileIndexing::Error> {
        let preview_result: Result<Option<PreviewedFile<'a>>, PreviewError> = path.preview().await;

        match preview_result {
            Ok(Some(p)) => { 
                // Preview successful
                let embedded_vector = p.calculate_embedding().await.map_err(|e| 
                    FileIndexing::Error { path: path.to_string(), source: e, r#type: FileIndexing::ErrorType::Embedding})?;
                let epoch_millis = p.timestamp.duration_since(SystemTime::UNIX_EPOCH)
                    .expect("timestamp is before linux epoch")
                    .as_millis().try_into().expect("linux epoch millis is more than usize");

                match self.vector_store.index(path.as_str(), embedded_vector, epoch_millis).await {
                    Ok(()) => Ok(FileIndexing::Result { path, r#type: FileIndexing::ResultType::Indexed }),
                    Err(e) => Err(FileIndexing::Error { path: path.to_string(), source: Box::new(e),
                        r#type: FileIndexing::ErrorType::Index }),
                }
            },
            Err(PreviewError::NotFound {..}) | Ok(None) => {
                // File not found or preview type not registered with preview system
                match self.vector_store.delete(path.as_str(), None).await {
                    Ok(()) => Ok(FileIndexing::Result { path: path, r#type: FileIndexing::ResultType::Cleared }),
                    Err(e) => Err(FileIndexing::Error { path: path.to_string(), source: Box::new(e),
                        r#type: FileIndexing::ErrorType::Clear }),
                }
            },
            Err(e) => {
                // Preview unable to be generated due to an error
                Err(FileIndexing::Error { path: path.to_string(), source: Box::new(e),
                    r#type: FileIndexing::ErrorType::Preview })
            },
        }
    }
    
    async fn index_multiple<'a>(&self, paths: Vec<&'a Utf8Path>) -> Vec<Result<FileIndexing::Result<'a>, FileIndexing::Error>> {
        let index_futures: Vec<_> = paths.iter().map(|f| self.index(f)).collect();
        join_all(index_futures).await
    }

    async fn clear<'a>(&self, path: &'a Utf8Path) -> Result<FileIndexing::Result<'a>, FileIndexing::Error> {
        // TODO - possibly deal with sequence number in some way?
        match self.vector_store.delete(path.as_str(), None).await {
            Ok(_) => Ok(FileIndexing::Result { path, r#type: FileIndexing::ResultType::Cleared }),
            Err(e) => Err(FileIndexing::Error { path: path.to_string(), source: Box::new(e),
                r#type: FileIndexing::ErrorType::Clear }),
        }
    }
}

pub mod FileIndexing {
    pub use super::result::{FileIndexingResult as Result, FileIndexingResultType as ResultType};
    pub use super::error::{FileIndexingError as Error, FileIndexingErrorType as ErrorType};
}
mod result;
mod error;