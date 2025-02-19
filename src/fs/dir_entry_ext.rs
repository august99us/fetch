use std::fs::DirEntry;

/// Extended helper functions for the DirEntry struct

/// Returns the file extension from the filename for a directory entry if it exists
pub fn retrieve_fileext(entry: &DirEntry) -> Result<String, &'static str> {
    match entry.path().extension() {
        Some(os_str) => os_str.to_owned().into_string().map_err(|_err| "Utf8 encoding error with file extension"),
        None => Ok(String::from("")), // if the file has no extension
    }
}