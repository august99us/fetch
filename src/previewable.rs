use std::time::SystemTime;

use camino::{Utf8Path, Utf8PathBuf};

pub enum PreviewType {
    Text,
    Image,
}
pub struct PreviewedFile {
    pub path: Utf8PathBuf,
    pub preview_path: Utf8PathBuf,
    pub timestamp: SystemTime,
    pub r#type: PreviewType,
}

/// Defines and implements the PossiblyPreviewable trait, representing a file that can potentially
/// be simplified or condensed into a smaller, limited size representation of the file. The maximum
/// size of these representations should be limited to <TODO> kb.
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
    async fn preview(&self) -> Result<Option<PreviewedFile>, PreviewError>;
}

#[derive(thiserror::Error, Debug)]
pub enum PreviewError {
    #[error("UTF8 Encoding Error")]
    Encoding { culprit: &'static str },
    #[error("File was not found")]
    NotFound { path: String },
    #[error("Error while generating preview")]
    Generation { path: String, #[source] source: anyhow::Error },
    #[error("Error interacting with file")]
    IO { path: String, #[source] source: std::io::Error },
}

impl PossiblyPreviewable for Utf8Path {
    async fn preview(&self) -> Result<Option<PreviewedFile>, PreviewError> {
        // check if preview is already available

        let extension = self.extension().unwrap_or("");

        let preview_path;

        if cache::os::has_generator_for_type(extension) {
            preview_path = cache::os::generate_preview(self).unwrap();
        } else {
            preview_path = cache::default::generate_preview(self).unwrap();
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