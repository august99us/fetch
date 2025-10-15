use std::error::Error;

use camino::Utf8Path;
use fetch_core::{app_config, file_index::{query_files::QueryFiles, FileIndexer}, previewable::PossiblyPreviewable, vector_store::lancedb_store::LanceDBStore};
use serde::Serialize;

use crate::utility::open_file_with_default_app;

#[derive(Debug, Serialize)]
pub struct QueryResult {
    pub name: String,
    pub path: String,
    pub score: f32,
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
async fn query(query: &str, page: u32) -> Result<Vec<QueryResult>, String> {
    let data_dir = app_config::get_default_index_directory();
    let lancedbstore = LanceDBStore::new(data_dir.as_str(), 768).await
        .unwrap_or_else(|e| panic!("Could not open lancedb store with data dir: {}. Error: {e:?}", data_dir.as_str()));
    let file_indexer = FileIndexer::with(lancedbstore);

    file_indexer.query_n(&query, 12, page)
        .await
        .map(|result| {
            result.into_iter()
                .map(|query_result| QueryResult {
                    name: query_result.path.file_name().expect("Result path should have a name").to_string(),
                    path: query_result.path.to_string(),
                    score: query_result.similarity,
                })
                .collect()
        })
        .map_err(|e| format!("{}, source: {}", e.to_string(), e.source.to_string()))
}

#[tauri::command]
async fn preview(path: &str) -> Result<Option<String>, String> {
    let path = Utf8Path::new(path);
    match path.preview().await {
        Ok(Some(previewed_file)) => Ok(Some(previewed_file.preview_path.to_string())),
        Ok(None) => Ok(None),
        Err(e) => Err(format!("Error while getting preview: {}", e.to_string())),
    }
}

#[tauri::command]
async fn open(path: &str) -> Result<(), String> {
    let path = Utf8Path::new(path);
    open_file_with_default_app(path)
        .map_err(|e| format!("{}, source: {}", e.to_string(), e.source().map(<dyn Error>::to_string).unwrap_or_default()))
}

#[tauri::command]
async fn open_location(path: &str) -> Result<(), String> {
    let path = Utf8Path::new(path);
    crate::utility::show_file_location(path)
        .map_err(|e| format!("{}, source: {}", e.to_string(), e.source().map(<dyn Error>::to_string).unwrap_or_default()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![query, open, open_location, preview])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub mod utility;