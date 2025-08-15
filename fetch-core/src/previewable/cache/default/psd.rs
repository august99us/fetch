use std::io::Cursor;

use image::{imageops::FilterType, DynamicImage, ImageFormat, RgbaImage};
use psd::{Psd, PsdLayer};
use tokio::{fs::File, io::AsyncReadExt, task};

pub const PREVIEW_EXTENSION: &str = "webp";

const PREVIEW_MAX_SIDE: u32 = 400;

/// Returns a vector of bytes representing a resized preview of the rendered psd file
pub async fn calculate_preview(mut file: File) -> Result<Vec<u8>, anyhow::Error> {
    let length = file.metadata().await?.len();
    let mut file_bytes: Vec<u8> = Vec::with_capacity(length as usize);
    file.read_to_end(&mut file_bytes).await?;

    let preview_bytes = task::spawn_blocking(move || {
        let psd = Psd::from_bytes(&file_bytes)?;

        let width = psd.width();
        let height = psd.height();
        // return true for all layers for now, include all layers in the flattened image
        let filter = &|(_i, _l): (usize, &PsdLayer)| true;
        let flattened_bytes = psd.flatten_layers_rgba(filter)?;

        let image = DynamicImage::from(RgbaImage::from_raw(width, height, flattened_bytes).unwrap());

        let image = image.resize(
            PREVIEW_MAX_SIDE,
            PREVIEW_MAX_SIDE,
            FilterType::Triangle,
        );

        let mut preview_bytes: Vec<u8> = Vec::new();
        image.write_to(&mut Cursor::new(&mut preview_bytes), ImageFormat::WebP)?;
        Ok::<Vec<u8>, anyhow::Error>(preview_bytes)
    }).await??; // this is Result<Result<vec, closure_error>, tokio::task_error>

    Ok(preview_bytes)
}