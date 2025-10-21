use std::error::Error;

use fetch_core::app_config;
use fetch_core::file_index::FileIndexer;
use fetch_core::vector_store::lancedb_store::LanceDBStore;

pub async fn get_file_indexer() -> Result<FileIndexer<LanceDBStore>, String> {
    let data_dir = app_config::get_default_index_directory();
    let lancedbstore = LanceDBStore::new(data_dir.as_str(), 768)
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
    Ok(FileIndexer::with(lancedbstore))
}
