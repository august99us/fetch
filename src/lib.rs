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

pub mod embeddable;
pub mod previewable;
pub mod index;