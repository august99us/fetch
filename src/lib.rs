use std::fs::File;
use std::error::Error;
use std::io::Bytes;
use std::time::SystemTime;

/// Library containing functionality to semantically translate files into multi-dimensional vectors
/// and then store those vectors in the fetch application index
pub enum PreviewType {
    Text,
    Image,
}
pub struct Preview<R> {
    content: Bytes<R>,
    path: String,
    timestamp: SystemTime,
    r#type: PreviewType,
}

mod embeddable;
mod previewable;
mod storage;

pub trait Indexer {
    // TODO: replace Box<dyn Error>
    fn index_file(&self, file: &File) -> Result<(), Box<dyn Error>>;
    fn index_files(&self, files: Vec<&File>) -> Result<(), Box<dyn Error>>;
}

pub struct SequentialIndexer {
    translators: HashMap<File, Translator>,
    semantic_store: VectorStore,
}
impl SequentialIndexer {
    pub fn new(translators, semantic_store) -> SequentialIndexer {
        SequentialIndexer {
            translators,
            semantic_store,
        }
    }
}
impl Indexer for SequentialIndexer {
    fn index_file(&self, file: &File) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
    fn index_files(&self, files: Vec<&File>) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

pub fn DefaultSequentialIndexer() -> SequentialIndexer {
    SequentialIndexer::new()
}