use std::{collections::HashMap, error::Error, fmt};

use camino::Utf8PathBuf;

use crate::index::provider::IndexProviderError;

// Cannot use thiserror::Error derive macros because all error enum types require a common
// path variable. There is probably a way to make it work in the thiserror library, but
// currently thiserror does not provide that functionality
#[derive(Debug)]
pub struct FileIndexingError {
    pub path: Utf8PathBuf,
    pub r#type: FileIndexingErrorType,
}

#[derive(Debug)]
pub enum FileIndexingErrorType {
    IndexProviders { provider_errors: HashMap<String, IndexProviderError> },
    Other { msg: &'static str, source: anyhow::Error },
}

impl fmt::Display for FileIndexingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.r#type {
            FileIndexingErrorType::IndexProviders { provider_errors } => {
                writeln!(f, "Error(s) occurred while indexing file at path: {}", self.path)?;
                for (provider_name, error) in provider_errors {
                    writeln!(f, "  Provider '{}': {}", provider_name, error)?;
                    error.fmt(f)?
                }
                Ok(())
            },
            FileIndexingErrorType::Other { msg, source } => {
                writeln!(f, "Error while indexing file at path: {} - {}", self.path, msg)?;
                source.fmt(f)
            },
        }
    }
}

impl Error for FileIndexingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.r#type {
            FileIndexingErrorType::Other { source, .. } => Some(&**source),
            _ => None,
        }
    }
}