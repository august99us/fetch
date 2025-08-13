use std::io;

use camino::{Utf8Path, Utf8PathBuf};
use fetch_core::{app_config, file_index::{query_files::QueryFiles, FileIndexer}, previewable::PossiblyPreviewable, vector_store::lancedb_store::LanceDBStore};
use iced::widget::image::Handle;
use tokio::{fs::File, io::AsyncReadExt};

pub async fn run_index_query(query: String) -> Result<Vec<Utf8PathBuf>, String> {
    let data_dir = app_config::get_default_index_directory();
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

pub async fn generate_or_retrieve_preview(path: &Utf8Path) -> Result<Option<Handle>, String> {
    match path.preview().await {
        Ok(Some(previewed_file)) => {
            let preview_path = previewed_file.preview_path;

            // load file
            let mut file = File::open(preview_path).await.map_err(map_io_error_to_string)?;
            let length = file.metadata().await.map_err(map_io_error_to_string)?.len();
            let mut file_bytes: Vec<u8> = Vec::with_capacity(length as usize);
            file.read_to_end(&mut file_bytes).await.map_err(map_io_error_to_string)?;

            Ok(Some(Handle::from_bytes(file_bytes)))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

fn map_io_error_to_string(e: io::Error) -> String {
    match e.kind() {
        _ => "IO Error".to_string(),
    }
}