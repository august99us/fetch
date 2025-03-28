use std::ops::Deref;
use std::{error::Error, fmt};
use std::time::SystemTime;

use camino::{Utf8Path, Utf8PathBuf};
use futures::future::join_all;
use semantic_index::{IndexPreview, QuerySimilarFiles};
use previewable::{PossiblyPreviewable, PreviewError};

/// Library containing functionality to semantically translate files into multi-dimensional vectors
/// and then store those vectors in the fetch application index
pub enum PreviewType {
    Text,
    Image,
}
pub struct PreviewedFile<'a> {
    path: &'a Utf8Path,
    preview_path: Utf8PathBuf,
    timestamp: SystemTime,
    r#type: PreviewType,
}

pub struct FileIndexer<I: IndexPreview + QuerySimilarFiles> {
    semantic_index: I,
}

// Perhaps this needs to be a struct so path can be a common variable amongst all variants?
pub enum FileIndexingResultType {
    Indexed,
    Cleared,
}
pub struct FileIndexingResult<'a> {
    pub path: &'a Utf8Path,
    pub r#type: FileIndexingResultType,
}

// Perhaps this needs to be a struct so path can be a common variable amongst all variants?
#[derive(Debug)]
pub struct FileIndexingError {
    pub path: String,
    pub source: Box<dyn Error>,
    pub r#type: FileIndexingErrorType,
}
#[derive(Debug)]
pub enum FileIndexingErrorType {
    Preview,
    Index,
    Clear,
}
impl fmt::Display for FileIndexingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        match self.r#type {
            FileIndexingErrorType::Preview => write!(f, "Unhandled error while generating or accessing \
                preview for file path {:?}", self.path),
            FileIndexingErrorType::Index => write!(f, "Error creating or updating index for file path {:?}", self.path),
            FileIndexingErrorType::Clear => write!(f, "Error clearing index for empty file path {:?}", self.path),
        }
    }
}
impl Error for FileIndexingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&*self.source)
    }
}

impl<I: IndexPreview + QuerySimilarFiles> FileIndexer<I> {
    pub fn new(index: I) -> FileIndexer<I> {
        FileIndexer { semantic_index: index }
    }

    pub async fn process_file<'a>(&self, file_path: &'a Utf8Path) -> Result<FileIndexingResult<'a>, FileIndexingError> {
        let preview_result = file_path.preview().await;

        match preview_result {
            Ok(Some(p)) => { 
                // Preview successful
                match self.semantic_index.index(p).await {
                    Ok(()) => Ok(FileIndexingResult { path: file_path, r#type: FileIndexingResultType::Indexed }),
                    Err(e) => Err(FileIndexingError { path: file_path.to_string(), source: Box::new(e),
                        r#type: FileIndexingErrorType::Index }),
                }
            },
            Err(PreviewError::NotFound {..}) | Ok(None) => {
                // File not found or preview type not registered with preview system
                match self.semantic_index.delete(file_path.as_str()).await {
                    Ok(()) => Ok(FileIndexingResult { path: file_path, r#type: FileIndexingResultType::Cleared }),
                    Err(e) => Err(FileIndexingError { path: file_path.to_string(), source: Box::new(e),
                        r#type: FileIndexingErrorType::Clear }),
                }
            },
            Err(e) => {
                // Preview unable to be generated due to an error
                Err(FileIndexingError { path: file_path.to_string(), source: Box::new(e), r#type: FileIndexingErrorType::Preview })
            },
        }
    }

    pub async fn process_files<'a>(&self, file_paths: Vec<&'a Utf8Path>) -> Vec<Result<FileIndexingResult<'a>, FileIndexingError>> {
        let process_futures: Vec<_> = file_paths.iter().map(|f| self.process_file(f)).collect();
        join_all(process_futures).await
    }
}

pub mod embeddable;
pub mod previewable;
pub mod semantic_index;