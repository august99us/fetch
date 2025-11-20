use std::io::Cursor;

use pdfium_render::prelude::{PdfPageRenderRotation, PdfRenderConfig};
use tokio::{fs::File, io::AsyncReadExt, task};

use crate::{environment::get_pdfium, previewable::cache::default::{PREVIEW_FORMAT, PREVIEW_MAX_SIDE}};

/// Returns a vector of bytes representing the preview image
pub async fn calculate_preview(mut file: File) -> Result<Vec<u8>, anyhow::Error> {
    let length = file.metadata().await?.len();
    let mut file_bytes: Vec<u8> = Vec::with_capacity(length as usize);
    file.read_to_end(&mut file_bytes).await?;

    let preview_bytes = task::spawn_blocking(move || {
        let pdfium = get_pdfium();
        let document = pdfium.load_pdf_from_byte_vec(file_bytes, None)?;

        let render_config = PdfRenderConfig::new()
            .scale_page_to_display_size(PREVIEW_MAX_SIDE as i32, PREVIEW_MAX_SIDE as i32)
            .rotate(PdfPageRenderRotation::None, false)
            .use_print_quality(false)
            .set_image_smoothing(false)
            .render_annotations(false)
            .render_form_data(false);

        let image = if let Ok(page) = document.pages().first() {
            page.render_with_config(&render_config)?
                .as_image()
        } else {
            return Err(anyhow::Error::msg("PDF did not have any pages"));
        };

        let mut preview_bytes: Vec<u8> = Vec::new();
        image.write_to(&mut Cursor::new(&mut preview_bytes), PREVIEW_FORMAT)?;
        Ok::<Vec<u8>, anyhow::Error>(preview_bytes)
    }).await??; // this is Result<Result<vec, closure_error>, tokio::task_error>

    Ok(preview_bytes)
}