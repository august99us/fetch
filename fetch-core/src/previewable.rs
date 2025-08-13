use std::time::SystemTime;

use camino::{Utf8Path, Utf8PathBuf};

/// The type of preview that was generated for a file.
pub enum PreviewType {
    /// A text-based preview of the file content
    Text,
    /// An image preview of the file (thumbnail, rendered preview, etc.)
    Image,
}
/// A file that has been successfully processed into a preview representation.
/// 
/// This struct contains metadata about both the original file and its generated preview,
/// including timing information and the type of preview that was created.
pub struct PreviewedFile {
    /// The path to the original file
    pub path: Utf8PathBuf,
    /// The path to the generated preview file
    pub preview_path: Utf8PathBuf,
    /// When this preview was generated
    pub timestamp: SystemTime,
    /// The type of preview that was generated
    pub r#type: PreviewType,
}

/// Describes an object that can potentially be simplified or condensed into a smaller, 
/// limited size preview representation.
/// 
/// Whether these limited representations can be constructed depends on two factors:
/// 1) whether the OS that the program is running on understands how to condense the file (whether
///    by default, or through some kind of installed plugin for the preview system - QuickLook on
///    OS X, Preview Handler on Windows, etc)
/// 2) whether Fetch has a default condenser for that type of file.
/// The OS provided preview system will always be preferred over the Fetch defaults, to better
/// facilitate the ability of the program to reuse the user's choices previously installed on the
/// operating system.
/// 
/// These limited representations will then be fed into Fetch's preview-to-semantic neural network
/// in order to generate semantic representations of the previews, which will be indexed and then
/// utilized to find semantically related files to a given input query.
pub trait PossiblyPreviewable {
    /// Attempt to generate a preview representation of this object.
    /// 
    /// # Returns
    /// 
    /// * `Ok(Some(PreviewedFile))` - A preview was successfully generated
    /// * `Ok(None)` - No preview could be generated (unsupported file type)
    /// * `Err(PreviewError)` - An error occurred during preview generation
    async fn preview(&self) -> Result<Option<PreviewedFile>, PreviewError>;
}

/// Errors that can occur during preview generation.
#[derive(thiserror::Error, Debug)]
pub enum PreviewError {
    /// A UTF-8 encoding error occurred during preview processing.
    #[error("UTF8 Encoding Error")]
    Encoding { culprit: &'static str },
    
    /// The file could not be found at the specified path.
    #[error("File was not found")]
    NotFound { path: String },
    
    /// An error occurred during the preview generation process.
    #[error("Error while generating preview")]
    Generation { path: String, #[source] source: anyhow::Error },
    
    /// An I/O error occurred while accessing the file.
    #[error("Error interacting with file")]
    IO { path: String, #[source] source: std::io::Error },
}

impl PossiblyPreviewable for Utf8Path {
    async fn preview(&self) -> Result<Option<PreviewedFile>, PreviewError> {
        // check if preview is already available

        let extension = self.extension().unwrap_or("");

        let preview_path;

        if cache::os::has_generator_for_type(extension) {
            preview_path = cache::os::generate_preview(self).await?;
        } else {
            preview_path = cache::default::generate_preview(self).await?;
        }

        Ok(preview_path.map(|pp| PreviewedFile {
            path: self.to_path_buf(),
            preview_path: pp.to_path_buf(),
            timestamp: SystemTime::now(),
            r#type: PreviewType::Image,
        }))
    }
}

// Private helper methods/modules?

mod cache;