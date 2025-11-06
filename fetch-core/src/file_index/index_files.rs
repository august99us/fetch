use std::future::Future;

use camino::Utf8Path;
use log::debug;

use super::FileIndexer;

/// Describes an object that understands how to semantically index files.
/// 
/// This trait provides methods for indexing files by first generating previews,
/// then creating embeddings from those previews, and finally storing them for later retrieval.
pub trait IndexFiles {
    /// Index a file, first attempting to load it, then generating a preview for the file, then embedding it, and
    /// finally indexing it into a vector store
    /// If the file does not exist or a preview is unable to be generated for the file, then the file is cleared
    /// from the index instead
    fn index<'a>(&self, path: &'a Utf8Path) -> impl Future<Output = Result<FileIndexingResult<'a>, FileIndexingError>> + Send;
    /// Clear the index for a file path. Does not check for the existence of the file
    fn clear<'a>(&self, path: &'a Utf8Path) -> impl Future<Output = Result<FileIndexingResult<'a>, FileIndexingError>> + Send;
    // Clears the index for all files currently indexed under a path. Does not check for existence of the path or files
    // EG. clear_fuzzy("/home/august99us/test") would clear "/home/august99us/test/dog.jpg" and "/home/august99us/test/cat.jpg"
    // as well as /home/august99us/test/testing/doc.pdf any other files that have /home/august99us/test in the path
    // TODO build this api
    // async fn clear_fuzzy(&self, path: &'a Utf8Path) -> Result<FileIndexing::Result<'a>, FileIndexing::Error>;
    // TODO perhaps this api too?
    // async fn index_fuzzy<'a>(&self, path: &'a Utf8Path) -> Result<FileIndexing::Result<'a>, FileIndexing::Error>;
}

impl IndexFiles for FileIndexer
{
    async fn index<'a>(&self, path: &'a Utf8Path) -> Result<FileIndexingResult<'a>, FileIndexingError> {
        debug!("FileIndexer: Indexing file with path: {}", path);
        let extension = path.extension().unwrap_or("");

        for provider in &self.index_providers {
            if provider.provides_indexing_for_extension(extension) {
                provider.index(path).await.map_err(|e| 
                    FileIndexingError {
                        path: path.to_owned(),
                        provider: get_type_name(&provider).to_owned(),
                        source: e
                    })?;
            }
        }

        Ok(FileIndexingResult { path, r#type: FileIndexingResultType::Indexed })
    }

    async fn clear<'a>(&self, path: &'a Utf8Path) -> Result<FileIndexingResult<'a>, FileIndexingError> {
        debug!("FileIndexer: Clearing index of path: {}", path);
        let extension = path.extension().unwrap_or("");
        
        for provider in &self.index_providers {
            if provider.provides_indexing_for_extension(extension) {
                // TODO - possibly deal with sequence number in some way?
                provider.clear(path).await.map_err(|e| 
                    FileIndexingError {
                        path: path.to_owned(),
                        provider: get_type_name(&provider).to_owned(),
                        source: e
                    })?;
            }
        }

        Ok(FileIndexingResult { path, r#type: FileIndexingResultType::Cleared })
    }
}

pub use result::*;
pub use error::*;

// private modules and functions

fn get_type_name<T>(_: &T) -> &'static str {
    std::any::type_name::<T>()
}

mod result;
mod error;