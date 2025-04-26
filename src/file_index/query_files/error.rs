use std::{error::Error, fmt};

// Cannot use thiserror::Error derive macros because all error enum types require a common
// query variable. There is probably a way to make it work in the thiserror library, but
// currently thiserror does not provide that functionality
#[derive(Debug)]
pub struct FileQueryingError {
    pub query: String,
    pub source: anyhow::Error,
    pub r#type: FileQueryingErrorType,
}
#[derive(Debug)]
pub enum FileQueryingErrorType {
    Embedding,
    Query,
}
impl fmt::Display for FileQueryingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        match self.r#type {
            FileQueryingErrorType::Embedding => write!(f, "Unhandled error while generating embedding for \
                file query {:?}", self.query),
            FileQueryingErrorType::Query => write!(f, "Error querying index with query {:?}", self.query),
        }
    }
}
impl Error for FileQueryingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&*self.source)
    }
}