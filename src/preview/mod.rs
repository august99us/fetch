pub mod previewable;
use std::io::Bytes;

pub use previewable::PossiblyPreviewable;

pub struct Preview {
    content: Bytes,
    preview_type: String,
}