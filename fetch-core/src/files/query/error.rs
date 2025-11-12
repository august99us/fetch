use std::{collections::HashMap, error::Error, fmt};

use crate::index::provider::IndexProviderError;

// Cannot use thiserror::Error derive macros because all error enum types require a common
// query variable. There is probably a way to make it work in the thiserror library, but
// currently thiserror does not provide that functionality
#[derive(Debug)]
pub struct FileQueryingError {
    pub query: String,
    pub r#type: FileQueryingErrorType,
}
#[derive(Debug)]
pub enum FileQueryingErrorType {
    CursorNotFound,
    CursorStore { source: anyhow::Error },
    IndexProviders { provider_errors: HashMap<String, IndexProviderError> },
    Other { msg: &'static str, source: anyhow::Error },
}
impl fmt::Display for FileQueryingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.r#type {
            FileQueryingErrorType::CursorNotFound => write!(f, "Cursor id could not be found. Try rerunning \
                the query from the start?"),
            FileQueryingErrorType::CursorStore { source } => {
                write!(f, "Error while interacting with the cursor store")?;
                source.fmt(f)
            }
            FileQueryingErrorType::IndexProviders { provider_errors } => {
                write!(f, "Error querying index with query {:?}", self.query)?;
                for (provider_name, error) in provider_errors {
                    writeln!(f, "  Provider '{}': {}", provider_name, error)?;
                }
                Ok(())
            }
            FileQueryingErrorType::Other { msg, source } => {
                write!(f, "Error querying index with query {:?} - {}", self.query, msg)?;
                source.fmt(f)
            }
        }
    }
}
impl Error for FileQueryingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.r#type {
            FileQueryingErrorType::CursorStore { source } => Some(&**source),
            FileQueryingErrorType::Other { source, .. } => Some(&**source),
            _ => None,
        }
    }
}