use super::Preview;
use std::fs::DirEntry;
use crate::fs::dir_entry_ext::retrieve_fileext;

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
    fn can_preview(&self) -> bool;
    fn preview(&self) -> Option<Preview>; // possibly this might need to be Result<Ok, Err>. need to reevaluate later.
}

impl PossiblyPreviewable for DirEntry {
    fn can_preview(&self) -> bool {
        let extension;
        match retrieve_fileext(self) {
            Ok(ext) => extension = ext,
            Err(_) => return false,
        }

        os_preview_generator::has_generator_for_type(&extension) || default_preview_generator::has_generator_for_type(&extension)
    }

    fn preview(&self) -> Option<Preview> {
        let extension;
        match retrieve_fileext(self) {
            Ok(ext) => extension = ext,
            Err(_) => return None,
        }

        if os_preview_generator::has_generator_for_type(&extension) {
            os_preview_generator::generate_preview(&self)
        } else if default_preview_generator::has_generator_for_type(&extension) {
            default_preview_generator::generate_preview(&self)
        } else {
            None
        }
    }
}

// Private helper methods/modules?

mod os_preview_generator;
mod default_preview_generator;