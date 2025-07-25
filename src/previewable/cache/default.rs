use std::{fs::{self, File}, hash::{DefaultHasher, Hash, Hasher}, io, collections::HashMap, sync::LazyLock};

use camino::{Utf8Path, Utf8PathBuf};

use crate::{app_config, previewable::{PreviewError, PreviewType}};

// Function interface, takes in a file, returns the bytes of the generated preview and its file extension
// in eg. "txt" format
type CalcFnPointer = fn(File) -> Result<Vec<u8>, anyhow::Error>;

static EXTENSION_TO_PREVIEW_TYPE: LazyLock<HashMap<&'static str, PreviewType>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert("jpg", PreviewType::Image);
    map.insert("png", PreviewType::Image);
    // Add more extensions and their corresponding preview calculation functions here
    map
});

pub fn has_generator_for_type(extension: &str) -> bool {
    EXTENSION_TO_PREVIEW_TYPE.contains_key(extension)
}

pub fn generate_preview(path: &Utf8Path) -> Result<Option<Utf8PathBuf>, PreviewError> {
    // verify that file_name of the path is valid
    if path.file_name().is_none() {
        return Err(PreviewError::NotFound { path: path.to_string() });
    }

    // Verify the file exists and open it
    let file = File::open(path).map_err(|e| -> PreviewError {
        match e.kind() {
            std::io::ErrorKind::NotFound => PreviewError::NotFound { path: path.to_string() },
            _ => PreviewError::IO { path: path.to_string(), source: e },
        }
    })?;

    // Check if the file has an extension that we can generate a preview for
    let extension = path.extension().unwrap_or("");
    if !has_generator_for_type(extension) {
        return Ok(None);
    }

    // At this point we know that 1) the file exists and 2) we should be able to generate a preview for it,
    // because the module has registered the fact that we have a generator for this file type.

    let preview_type = EXTENSION_TO_PREVIEW_TYPE.get(extension)
        .expect("Already checked that extension is something this module can operate on");

    // TODO: Locking when the try_lock() API is available to stable rust std?

    // First check if the preview is already available in the cache
    let preview_filename = hash_file_path(path, preview_type_to_extension(preview_type));
    let preview_path = retrieve_preview_directory().join(preview_filename);
    if preview_path.is_file() {
        let preview_file = File::open(&preview_path)
            .expect(format!("Could not open preview file even though .is_file() succeeded: {}", preview_path).as_str());
        if preview_creation_after_file_modification(&file, &preview_file)
            .map_err(|e| PreviewError::IO { path: path.to_string(), source: e })? {
            return Ok(Some(preview_path));
        }
    }

    // preview is not available or outdated so it needs to be re-generated

    let bytes = (preview_type_to_calculate_fn(preview_type))(file)
        .map_err(|e| PreviewError::Generation { path: path.to_string(), source: e })?;
    fs::write(&preview_path, &bytes)
        .map_err(|e| PreviewError::IO { path: path.to_string(), source: e })?;

    Ok(Some(preview_path))
}

// private functions/modules

// Returns a function pointer to the calculation function for the given preview type
// Expects that this mapping existing was something that was previously checked
fn preview_type_to_calculate_fn(preview_type: &PreviewType) -> CalcFnPointer {
    match preview_type {
        PreviewType::Image => image::calculate_preview as CalcFnPointer,
        // Add more preview types and their corresponding calculation functions here
        _ => panic!("No calculation function registered for this preview type"),
    }
}

// Returns the preview file extension for the given preview type
// Expects that this mapping existing was something that was previously checked
fn preview_type_to_extension(preview_type: &PreviewType) -> &'static str {
    match preview_type {
        PreviewType::Image => image::PREVIEW_EXTENSION,
        // Add more preview types and their corresponding extensions here
        _ => panic!("No extension registered for this preview type"),
    }
}

// checks if the preview file was created after the original file was modified
fn preview_creation_after_file_modification(file: &File, preview_file: &File) -> Result<bool, io::Error> {
    let file_modified = file.metadata()?.modified()?.duration_since(std::time::UNIX_EPOCH)
        .expect("File modified time should be after UNIX_EPOCH");
    let preview_created = preview_file.metadata()?.created()?.duration_since(std::time::UNIX_EPOCH)
        .expect("Preview created time should be after UNIX_EPOCH");

    Ok(preview_created > file_modified)
}

fn retrieve_preview_directory() -> Utf8PathBuf {
    app_config::get_default_preview_directory()
}

// Hash file path. Expects that path.file_name() results in a valid UTF-8 string. Will panic otherwise.
fn hash_file_path(path: &Utf8Path, preview_extension: &str) -> String {
    let mut hasher = DefaultHasher::new();
    path.as_str().hash(&mut hasher);
    format!("{:x}-{}.{}", hasher.finish(), 
        path.file_stem().expect("file_stem() should be previously checked, cannot be None"), 
        preview_extension)
}

mod image;