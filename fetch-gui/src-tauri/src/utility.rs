use std::error::Error;
use std::sync::Arc;

use fetch_core::app_config;
use fetch_core::file_index::pagination::QueryCursor;
use fetch_core::file_index::{FileIndexer, FileQueryer};
use fetch_core::index::basic_image_index_provider::BasicImageIndexProvider;
use fetch_core::store::lancedb::LanceDBStore;

pub async fn get_file_queryer() -> Result<FileQueryer<LanceDBStore<QueryCursor>>, String> {
    let data_dir = app_config::get_default_index_directory();
    // Create siglip store
    let siglip2_image_index = LanceDBStore::local_full(
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
        })?;
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
    let basic_image = BasicImageIndexProvider::using(siglip2_image_index);
    Ok(FileQueryer::with(vec![Arc::new(basic_image)], cursor_store))
}

pub async fn get_file_indexer() -> Result<FileIndexer, String> {
    let data_dir = app_config::get_default_index_directory();
    let siglip2_image_index = LanceDBStore::local_full(
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
        })?;
    let basic_image = BasicImageIndexProvider::using(siglip2_image_index);
    Ok(FileIndexer::with(vec![Arc::new(basic_image)]))
}
