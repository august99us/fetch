use std::error::Error;
use std::sync::Arc;

use fetch_core::app_config;
use fetch_core::files::pagination::QueryCursor;
use fetch_core::files::{FileIndexer, FileQueryer};
use fetch_core::index::provider::image::ImageIndexProvider;
use fetch_core::index::provider::pdf::PdfIndexProvider;
use fetch_core::store::lancedb::LanceDBStore;

pub async fn get_file_queryer() -> Result<FileQueryer<LanceDBStore<QueryCursor>>, String> {
    let data_dir = app_config::get_default_index_directory();
    // Create siglip store
    let siglip2_image_index = Arc::new(LanceDBStore::local_full(
            data_dir.as_str(),
            "siglip2_chunkfile".to_string()
        )
        .await
        .map_err(|e| {
            format!(
                "Could not open lancedb store: {}, source: {}",
                e,
                e.source()
                    .map(<dyn Error>::to_string)
                    .unwrap_or("".to_string())
            )
        })?);
    let gemma_text_index = Arc::new(LanceDBStore::local_full(
            data_dir.as_str(),
            "gemma_chunkfile".to_string()
        )
        .await
        .map_err(|e| {
            format!(
                "Could not open lancedb store: {}, source: {}",
                e,
                e.source()
                    .map(<dyn Error>::to_string)
                    .unwrap_or("".to_string())
            )
        })?);
    // Create the cursor store
    let cursor_store = LanceDBStore::<QueryCursor>::local(
            data_dir.as_str(),
            "cursor".to_owned()
        ).await
        .map_err(|e| {
            format!(
                "Could not open lancedb store for cursors: {}. source: {}",
                e,
                e.source()
                    .map(<dyn Error>::to_string)
                    .unwrap_or("".to_string())
            )
        })?;
    let basic_image = ImageIndexProvider::using(siglip2_image_index.clone());
    let pdf = PdfIndexProvider::using(gemma_text_index, siglip2_image_index);
    Ok(FileQueryer::with(vec![Arc::new(basic_image), Arc::new(pdf)], cursor_store))
}

pub async fn get_file_indexer() -> Result<FileIndexer, String> {
    let data_dir = app_config::get_default_index_directory();
    let siglip2_image_index = Arc::new(LanceDBStore::local_full(
            data_dir.as_str(),
            "siglip2_chunkfile".to_string()
        )
        .await
        .map_err(|e| {
            format!(
                "Could not open lancedb store: {}, source: {}",
                e,
                e.source()
                    .map(<dyn Error>::to_string)
                    .unwrap_or("".to_string())
            )
        })?);
    let gemma_text_index = Arc::new(LanceDBStore::local_full(
            data_dir.as_str(),
            "gemma_chunkfile".to_string()
        )
        .await
        .map_err(|e| {
            format!(
                "Could not open lancedb store: {}, source: {}",
                e,
                e.source()
                    .map(<dyn Error>::to_string)
                    .unwrap_or("".to_string())
            )
        })?);
    let basic_image = ImageIndexProvider::using(siglip2_image_index.clone());
    let pdf = PdfIndexProvider::using(gemma_text_index, siglip2_image_index);
    Ok(FileIndexer::with(vec![Arc::new(basic_image), Arc::new(pdf)]))
}
