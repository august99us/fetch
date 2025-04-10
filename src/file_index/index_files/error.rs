use std::{error::Error, fmt};

// Cannot use thiserror::Error derive macros because all error enum types require a common
// path variable. There is probably a way to make it work in the thiserror library, but
// currently thiserror does not provide that functionality
#[derive(Debug)]
pub struct FileIndexingError {
    pub path: String,
    pub source: Box<dyn Error>,
    pub r#type: FileIndexingErrorType,
}
#[derive(Debug)]
pub enum FileIndexingErrorType {
    Preview,
    Embedding,
    Index,
    Clear,
}
impl fmt::Display for FileIndexingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        match self.r#type {
            FileIndexingErrorType::Preview => write!(f, "Unhandled error while generating or accessing \
                preview for file path {:?}", self.path),
            FileIndexingErrorType::Embedding => write!(f, "Unhandled error while generating embedding for \
                file path {:?}", self.path),
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