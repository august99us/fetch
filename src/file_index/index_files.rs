use std::{future::Future, time::SystemTime};

use camino::Utf8Path;

use crate::{embeddable::Embeddable, previewable::{PossiblyPreviewable, PreviewError, PreviewedFile}, vector_store::{IndexVector, QueryVectorKeys}};

use super::FileIndexer;

/// index
pub trait IndexFiles {
    /// Index a file, first attempting to load it, then generating a preview for the file, then embedding it, and
    /// finally indexing it into a vector store
    /// If the file does not exist or a preview is unable to be generated for the file, then the file is cleared
    /// from the index instead
    fn index<'a>(&self, path: &'a Utf8Path) -> impl Future<Output = Result<FileIndexing::Result<'a>, FileIndexing::Error>> + Send;
    /// Clear the index for a file path. Does not check for the existence of the file
    async fn clear<'a>(&self, path: &'a Utf8Path) -> Result<FileIndexing::Result<'a>, FileIndexing::Error>;
    // Clears the index for all files currently indexed under a path. Does not check for existence of the path or files
    // EG. clear_fuzzy("/home/august99us/test_imgs") would clear "/home/august99us/test_imgs/dog.jpg" and "/home/august99us/test_imgs/cat.jpg"
    // as well as any other files that have /home/august99us/test_imgs in the path
    // TODO build this api
    // async fn clear_fuzzy(&self, path: &'a Utf8Path) -> Result<FileIndexing::Result<'a>, FileIndexing::Error>;
}

impl<I: IndexVector + QueryVectorKeys + Send + Sync> IndexFiles for FileIndexer<I> {
    async fn index<'a>(&self, path: &'a Utf8Path) -> Result<FileIndexing::Result<'a>, FileIndexing::Error> {
        let preview_result: Result<Option<PreviewedFile<'a>>, PreviewError> = path.preview().await;

        match preview_result {
            Ok(Some(p)) => { 
                // Preview successful
                let embedded_vector = p.calculate_embedding().await.map_err(|e| 
                    FileIndexing::Error { path: path.to_string(), source: e.into(), r#type: FileIndexing::ErrorType::Embedding})?;
                let epoch_millis = p.timestamp.duration_since(SystemTime::UNIX_EPOCH)
                    .expect("timestamp is before linux epoch")
                    .as_millis().try_into().expect("linux epoch millis is more than usize");

                match self.vector_store.index(path.as_str(), embedded_vector, epoch_millis).await {
                    Ok(()) => Ok(FileIndexing::Result { path, r#type: FileIndexing::ResultType::Indexed }),
                    Err(e) => Err(FileIndexing::Error { path: path.to_string(), source: e.into(),
                        r#type: FileIndexing::ErrorType::Index }),
                }
            },
            Err(PreviewError::NotFound {..}) | Ok(None) => {
                // File not found or preview type not registered with preview system
                match self.vector_store.delete(path.as_str(), None).await {
                    Ok(()) => Ok(FileIndexing::Result { path, r#type: FileIndexing::ResultType::Cleared }),
                    Err(e) => Err(FileIndexing::Error { path: path.to_string(), source: e.into(),
                        r#type: FileIndexing::ErrorType::Clear }),
                }
            },
            Err(e) => {
                // Preview unable to be generated due to an error
                Err(FileIndexing::Error { path: path.to_string(), source: e.into(),
                    r#type: FileIndexing::ErrorType::Preview })
            },
        }
    }

    async fn clear<'a>(&self, path: &'a Utf8Path) -> Result<FileIndexing::Result<'a>, FileIndexing::Error> {
        // TODO - possibly deal with sequence number in some way?
        match self.vector_store.delete(path.as_str(), None).await {
            Ok(_) => Ok(FileIndexing::Result { path, r#type: FileIndexing::ResultType::Cleared }),
            Err(e) => Err(FileIndexing::Error { path: path.to_string(), source: e.into(),
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