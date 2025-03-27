use std::error::Error;
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
pub struct Preview<'a> {
    // The naming of these member variables can make 
    path: Utf8PathBuf,
    original_file_path: &'a Utf8Path,
    timestamp: SystemTime,
    r#type: PreviewType,
}

pub struct FileIndexer<I: IndexPreview + QuerySimilarFiles> {
    semantic_index: I,
}

// Perhaps this needs to be a struct so path can be a common variable amongst all variants?
pub enum FileIndexingResult<'a> {
    Indexed{ path: &'a Utf8Path },
    Cleared{ path: &'a Utf8Path },
}

// Perhaps this needs to be a struct so path can be a common variable amongst all variants?
#[derive(thiserror::Error, Debug)]
pub enum FileIndexingError<'a> {
    #[error("Unhandled error while generating or accessing preview for file path {path:?}")]
    Preview { path: &'a Utf8Path, #[source] source: Box<dyn Error> },
    #[error("Error creating or updating index for file path {path:?}")]
    Index { path: &'a Utf8Path, #[source] source: Box<dyn Error> },
    #[error("Error clearing index for empty file path {path:?}")]
    Clear { path: &'a Utf8Path, #[source] source: Box<dyn Error> },
}

impl<I: IndexPreview + QuerySimilarFiles> FileIndexer<I> {
    pub fn new(index: I) -> FileIndexer<I> {
        FileIndexer { semantic_index: index }
    }

    pub async fn process_file<'a>(&self, file_path: &'a Utf8Path) -> Result<FileIndexingResult<'a>, FileIndexingError<'a>> {
        let preview_result = file_path.preview().await;

        match preview_result {
            Ok(Some(p)) => { 
                match self.semantic_index.index(p).await {
                    Ok(()) => Ok(FileIndexingResult::Indexed{ path: file_path }),
                    Err(e) => Err(FileIndexingError::Index { path: file_path, source: Box::new(e) }),
                }
            },
            Err(PreviewError::NotFound {..}) | Ok(None) => {
                match self.semantic_index.delete(file_path.as_str()).await {
                    Ok(()) => Ok(FileIndexingResult::Cleared{ path: file_path }),
                    Err(e) => Err(FileIndexingError::Clear { path: file_path, source: Box::new(e) }),
                }
            },
            Err(e) => {
                Err(FileIndexingError::Preview { path: file_path, source: Box::new(e) })
            },
        }
    }

    pub async fn process_files<'a>(&self, file_paths: Vec<&'a Utf8Path>) -> Vec<Result<FileIndexingResult<'a>, FileIndexingError<'a>>> {
        let process_futures: Vec<_> = file_paths.iter().map(|f| self.process_file(f)).collect();
        join_all(process_futures).await
    }
}

pub mod embeddable;
pub mod previewable;
pub mod semantic_index;