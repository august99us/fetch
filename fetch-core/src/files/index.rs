use std::{collections::HashMap, future::Future};

use camino::Utf8Path;
use chrono::{DateTime, Utc};
use log::{debug, info};

use crate::{files::ChunkingIndexProviderConcurrent, index::provider::IndexProviderErrorType};

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
    fn index<'a>(&self, path: &'a Utf8Path, opt_modified: Option<DateTime<Utc>>) -> impl Future<Output = Result<FileIndexingResult<'a>, FileIndexingError>> + Send;
    /// Clear the index for a file path. Does not check for the existence of the file
    fn clear<'a>(&self, path: &'a Utf8Path, opt_modified: Option<DateTime<Utc>>) -> impl Future<Output = Result<FileIndexingResult<'a>, FileIndexingError>> + Send;
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
    async fn index<'a>(&self, path: &'a Utf8Path, opt_modified: Option<DateTime<Utc>>) -> Result<FileIndexingResult<'a>, FileIndexingError> {
        debug!("FileIndexer: Indexing file with path: {}", path);

        let path_clone = path.to_owned();
        let results = self.index_providers.distribute_calls(async move |p| {
            let ext = path_clone.extension().unwrap_or("");
            if p.provides_indexing_for_extension(ext) {
                Some(p.index(&path_clone, opt_modified).await)
            } else {
                None
            }
        }).await.map_err(|e| FileIndexingError {
            path: path.to_owned(),
            r#type: FileIndexingErrorType::Other {
                msg: "Join error occurred while indexing file",
                source: e,
            },
        })?;

        let mut was_processed = false;
        let mut provider_error_map = HashMap::new();
        for res_opt in results {
            if let Some(res) = res_opt {
                was_processed = true;
                if let Err(e) = res {
                    let provider_name = e.provider_name.clone();
                    match e.r#type {
                        IndexProviderErrorType::Sequencing { provided_datetime, stored_datetime } => {
                            // Ignore sequencing errors.
                            info!("FileIndexer: Attempted indexing on file: {} but the stored modified_date \
                                ({}) was equal to or later than the file's modified_date ({}). Ignoring.",
                                path,
                                stored_datetime, provided_datetime
                            );
                        },
                        _ => {
                            provider_error_map.insert(provider_name, e);
                        }
                    }
                }
            }
        }

        if !was_processed {
            return Ok(FileIndexingResult { path, r#type: FileIndexingResultType::Skipped {
                reason: "Extension not registered in any provider".to_string() } })
        }
        
        if !provider_error_map.is_empty() {
            return Err(FileIndexingError { path: path.to_owned(), r#type: FileIndexingErrorType::IndexProviders {
                provider_errors: provider_error_map,
            }});
        }

        Ok(FileIndexingResult { path, r#type: FileIndexingResultType::Indexed })
    }

    async fn clear<'a>(&self, path: &'a Utf8Path, opt_modified: Option<DateTime<Utc>>) -> Result<FileIndexingResult<'a>, FileIndexingError> {
        debug!("FileIndexer: Clearing index of path: {}", path);

        let path_clone = path.to_owned();
        let results = self.index_providers.distribute_calls(async move |p| {
            let ext = path_clone.extension().unwrap_or("");
            if p.provides_indexing_for_extension(ext) {
                p.clear(&path_clone, opt_modified).await
            } else {
                Ok(())
            }
        }).await.map_err(|e| FileIndexingError {
            path: path.to_owned(),
            r#type: FileIndexingErrorType::Other {
                msg: "Join error occurred while indexing file",
                source: e,
            },
        })?;

        let mut provider_error_map = HashMap::new();
        for res in results {
            if let Err(e) = res {
                let provider_name = e.provider_name.clone();
                match e.r#type {
                    IndexProviderErrorType::InvalidExtension { path } => {
                        // Ignore invalid extension errors on clear
                        info!("FileIndexer: Attempted clear on file: {} but extension was invalid. Ignoring.",
                            path,
                        );
                    }
                    IndexProviderErrorType::Sequencing { provided_datetime, stored_datetime } => {
                        // Ignore sequencing errors.
                        info!("FileIndexer: Attempted clear on file: {} but the stored modified_date \
                            ({}) was equal to or later than the file's modified_date ({}). Ignoring.",
                            path,
                            stored_datetime, provided_datetime
                        );
                    },
                    _ => {
                        provider_error_map.insert(provider_name, e);
                    }
                }
            }
        }
        
        if !provider_error_map.is_empty() {
            return Err(FileIndexingError { path: path.to_owned(), r#type: FileIndexingErrorType::IndexProviders {
                provider_errors: provider_error_map,
            }});
        }

        Ok(FileIndexingResult { path, r#type: FileIndexingResultType::Cleared })
    }
}

pub use result::*;
pub use error::*;

// private modules and functions

mod result;
mod error;