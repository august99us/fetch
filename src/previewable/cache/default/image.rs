use std::{fs::File, io::{BufReader, Cursor}};

use image::{imageops::FilterType, ImageFormat, ImageReader};

pub const PREVIEW_EXTENSION: &str = "webp";

const PREVIEW_MAX_SIDE: u32 = 400;

// Returns a vector of bytes representing the preview image and its file extension
pub fn calculate_preview(file: File) -> Result<Vec<u8>, anyhow::Error> {
    let image = ImageReader::new(BufReader::new(file))
        .with_guessed_format()?
        .decode()?;

    let image = image.resize(
        PREVIEW_MAX_SIDE,
        PREVIEW_MAX_SIDE,
        FilterType::Triangle,
    );

    let mut bytes: Vec<u8> = Vec::new();
    image.write_to(&mut Cursor::new(&mut bytes), ImageFormat::WebP)?;

    Ok(bytes)
}