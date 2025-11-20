use std::{collections::HashMap, future::Future, hash::{DefaultHasher, Hash, Hasher}, io, pin::Pin, sync::LazyLock};

use camino::{Utf8Path, Utf8PathBuf};
use ::image::ImageFormat;
use log::info;
use tokio::fs::{self, File};

use crate::{app_config, previewable::PreviewError};

pub fn has_generator_for_type(extension: &str) -> bool {
    EXTENSION_TO_FUNCTION.contains_key(extension)
}

pub async fn generate_preview(path: &Utf8Path) -> Result<Option<Utf8PathBuf>, PreviewError> {
    // verify that file_name of the path is valid
    if path.file_name().is_none() {
        return Err(PreviewError::NotFound { path: path.to_string() });
    }

    // Verify the file exists and open it
    let file = File::open(path).await.map_err(|e| -> PreviewError {
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
    let preview_fn = EXTENSION_TO_FUNCTION.get(extension)
        .expect("Already checked that extension is something this module can operate on");

    // TODO: Locking when the try_lock() API is available to stable rust std?

    // First check if the preview is already available in the cache
    let preview_filename = hash_file_path(path);
    let preview_path = retrieve_preview_directory().join(preview_filename);
    if preview_path.is_file() {
        let preview_file = File::open(&preview_path).await
            .unwrap_or_else(|_| panic!("Could not open preview file even though .is_file() succeeded: {}", preview_path));
        if preview_creation_after_file_modification(&file, &preview_file).await
            .map_err(|e| PreviewError::IO { path: path.to_string(), source: e })? {
            return Ok(Some(preview_path));
        }
    }

    // preview is not available or outdated so it needs to be re-generated

    let bytes = preview_fn(file).await
        .map_err(|e| PreviewError::Generation { path: path.to_string(), source: e })?;
    fs::write(&preview_path, &bytes).await
        .map_err(|e| PreviewError::IO { path: path.to_string(), source: e })?;

    info!("Generated preview for file: {} at {}", path, preview_path);

    Ok(Some(preview_path))
}

// private functions/modules/constant

// max height/width for generated previews. to be imported in submodules
const PREVIEW_MAX_SIDE: u32 = 300;

const PREVIEW_FORMAT: ImageFormat = ImageFormat::WebP;
const PREVIEW_FILE_EXTENSION: &str = "webp";

// Function interface, takes in a file, returns the bytes of the generated preview and its file extension
// in eg. "txt" format
type CalcFnPointer = fn(File) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, anyhow::Error>> + Send>>;

static EXTENSION_TO_FUNCTION: LazyLock<HashMap<&'static str, CalcFnPointer>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    // file types supported by image crate
    let standard_image_fn = (|f| Box::pin(image::calculate_preview(f))) as CalcFnPointer;
    map.insert("avif", standard_image_fn);
    map.insert("bmp", standard_image_fn);
    map.insert("dds", standard_image_fn);
    map.insert("ff", standard_image_fn);
    map.insert("hdr", standard_image_fn);
    map.insert("ico", standard_image_fn);
    map.insert("jpg", standard_image_fn);
    map.insert("jpeg", standard_image_fn);
    map.insert("exr", standard_image_fn);
    map.insert("png", standard_image_fn);
    map.insert("pnm", standard_image_fn);
    map.insert("qoi", standard_image_fn);
    map.insert("tga", standard_image_fn);
    map.insert("tif", standard_image_fn);
    map.insert("tiff", standard_image_fn);
    map.insert("webp", standard_image_fn);
    // psd files, using psd crate
    #[cfg(feature = "psd")]
    {
        let psd_image_fn = (|f| Box::pin(psd::calculate_preview(f))) as CalcFnPointer;
        map.insert("psd", psd_image_fn);
    }
    #[cfg(feature = "pdf")]
    {
        let pdf_image_fn = (|f| Box::pin(pdf::calculate_preview(f))) as CalcFnPointer;
        map.insert("pdf", pdf_image_fn);
    }
    // Add more extensions and their corresponding preview calculation functions here
    map
});

// checks if the preview file was created after the original file was modified
async fn preview_creation_after_file_modification(file: &File, preview_file: &File) -> Result<bool, io::Error> {
    let file_modified = file.metadata().await?.modified()?.duration_since(std::time::UNIX_EPOCH)
        .expect("File modified time should be after UNIX_EPOCH");
    let preview_created = preview_file.metadata().await?.created()?.duration_since(std::time::UNIX_EPOCH)
        .expect("Preview created time should be after UNIX_EPOCH");

    Ok(preview_created > file_modified)
}

fn retrieve_preview_directory() -> Utf8PathBuf {
    app_config::get_default_preview_directory()
}

// Hash file path. Expects that path.file_name() results in a valid UTF-8 string. Will panic otherwise.
fn hash_file_path(path: &Utf8Path) -> String {
    let mut hasher = DefaultHasher::new();
    path.as_str().hash(&mut hasher);
    format!("{:x}-{}.{}", hasher.finish(), 
        path.file_stem().expect("file_stem() should be previously checked, cannot be None"), 
        PREVIEW_FILE_EXTENSION)
}

mod image;
#[cfg(feature = "psd")]
mod psd;
#[cfg(feature = "pdf")]
mod pdf;