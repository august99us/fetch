
use std::{error::Error, fmt};

use camino::Utf8PathBuf;
use chrono::{DateTime, Utc};

use crate::index::embedding::EmbeddingError;

// Cannot use thiserror::Error derive macros because all error enum types require a common
// query variable. There is probably a way to make it work in the thiserror library, but
// currently thiserror does not provide that functionality
#[derive(Debug)]
pub struct IndexProviderError {
    pub provider_name: String,
    pub r#type: IndexProviderErrorType,
}
#[derive(Debug)]
pub enum IndexProviderErrorType {
    InvalidExtension { path: Utf8PathBuf },
    Sequencing { provided_datetime: DateTime<Utc>, stored_datetime: DateTime<Utc> },
    IO { path: String, source: anyhow::Error },
    Chunking { path: String, source: anyhow::Error },
    Embedding { source: EmbeddingError },
    Store { operation: &'static str, source: anyhow::Error },
    Unknown { msg: &'static str, source: anyhow::Error },
}
impl fmt::Display for IndexProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.r#type {
            IndexProviderErrorType::InvalidExtension { path } => 
                write!(f, "Indexing not provided for files with this extension: {}", path),
            IndexProviderErrorType::Sequencing { provided_datetime, stored_datetime } => 
                write!(f, "File modified datetime ({}) is equal to or earlier than previously indexed version ({})",
                    provided_datetime, stored_datetime),
            IndexProviderErrorType::IO { path, source } => {
                write!(f, "Error occurred while interacting with filesystem at path: {}", path)?;
                source.fmt(f)
            },
            IndexProviderErrorType::Chunking { path, source } => {
                write!(f, "Error occurred while chunking file at path: {}", path)?;
                source.fmt(f)
            },
            IndexProviderErrorType::Embedding { source } => {
                write!(f, "Error occurred while embedding file or query")?;
                source.fmt(f)
            },
            IndexProviderErrorType::Store { operation, source } => {
                write!(f, "Error occurred while interacting with database during operation: {}", operation)?;
                source.fmt(f)
            },
            IndexProviderErrorType::Unknown { msg, source } => {
                write!(f, "Error: {}", msg)?;
                source.fmt(f)
            },
        }
    }
}
impl Error for IndexProviderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.r#type {
            IndexProviderErrorType::IO { source, .. } => Some(&**source),
            IndexProviderErrorType::Chunking { source, .. } => Some(&**source),
            IndexProviderErrorType::Embedding { source, .. } => Some(source),
            IndexProviderErrorType::Store { source, .. } => Some(&**source),
            IndexProviderErrorType::Unknown { source, .. } => Some(&**source),
            _ => None,
        }
    }
}