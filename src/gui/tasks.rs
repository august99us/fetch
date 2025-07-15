use camino::{Utf8Path, Utf8PathBuf};
use fetch::{app_config, file_index::{query_files::QueryFiles, FileIndexer}, previewable::PossiblyPreviewable, vector_store::lancedb_store::LanceDBStore};

pub async fn run_index_query(query: String) -> Result<Vec<Utf8PathBuf>, String> {
    let data_dir = app_config::get_default_data_directory();
    let lancedbstore = LanceDBStore::new(data_dir.as_str(), 512).await
        .unwrap_or_else(|e| panic!("Could not open lancedb store with data dir: {}. Error: {e:?}", data_dir.as_str()));
    let file_indexer = FileIndexer::with(lancedbstore);

    file_indexer.query(&query)
        .await
        .map(|result| {
            result.into_iter()
                .map(|query_result| query_result.path)
                .collect::<Vec<Utf8PathBuf>>()
        })
        .map_err(|e| e.to_string())
}

pub async fn generate_or_retrieve_preview(path: Utf8PathBuf) -> Result<Option<Utf8PathBuf>, String> {
    match path.preview().await {
        Ok(Some(previewed_file)) => Ok(Some(previewed_file.preview_path)),
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}