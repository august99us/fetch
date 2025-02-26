use std::{fs::DirEntry, io::Bytes};

pub enum PreviewType {
    Text,
    Image,
}
pub struct Preview<R> {
    content: Bytes<R>,
    path: DirEntry,
    r#type: PreviewType,
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
pub trait PossiblyPreviewable<R> {
    fn preview(&self) -> Result<Option<Preview<R>>, &str>;
}

impl<R> PossiblyPreviewable<R> for DirEntry {
    fn preview(&self) -> Result<Option<Preview<R>>, &str> {
        let extension;
        match retrieve_file_ext(self) {
            Ok(ext) => extension = ext,
            Err(e) => return Err(e),
        }

        if os_preview_generator::has_generator_for_type(&extension) {
            os_preview_generator::generate_preview(&self).map(|p| Some(p))
        } else if default_preview_generator::has_generator_for_type(&extension) {
            default_preview_generator::generate_preview(&self).map(|p| Some())
        } else {
            Ok(None)
        }
    }
}

// Private helper methods/modules?

mod os_preview_generator;
mod default_preview_generator;

/// Returns the file extension from the filename for a directory entry if it exists
/// Can return an empty string "" (if the file does not have an extension)
/// 
/// Errors if the file extension cannot be decoded into utf8 properly
pub fn retrieve_file_ext(entry: &DirEntry) -> Result<String, &'static str> {
    match entry.path().extension() {
        Some(os_str) => os_str.to_owned().into_string().map_err(|_err| "Utf8 encoding error with file extension"),
        None => Ok(String::from("")), // if the file has no extension
    }
}