use camino::Utf8PathBuf;

#[derive(thiserror::Error, Debug)]
#[error("Error occured while indexing file")]
pub struct FileIndexingError {
    /// The path that the FileIndex was indexing when the error occured
    pub path: Utf8PathBuf,
    /// The name of the IndexProvider that caused the error
    pub provider: String,
    #[source]
    pub source: anyhow::Error,
}