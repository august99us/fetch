use std::error::Error;

use camino::{Utf8Path, Utf8PathBuf};
use fetch_core::{
    app_config,
    file_index::{index_files::IndexFiles, query_files::QueryFiles, FileIndexer},
    previewable::PossiblyPreviewable,
    vector_store::lancedb_store::LanceDBStore,
};
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
    let lancedbstore = LanceDBStore::new(data_dir.as_str(), 768)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "Could not open lancedb store with data dir: {}. Error: {e:?}",
                data_dir.as_str()
            )
        });
    let file_indexer = FileIndexer::with(lancedbstore);

    file_indexer
        .query_n(&query, 12, page)
        .await
        .map(|result| {
            result
                .into_iter()
                .map(|query_result| QueryResult {
                    name: query_result
                        .path
                        .file_name()
                        .expect("Result path should have a name")
                        .to_string(),
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
    open_file_with_default_app(path).map_err(|e| {
        format!(
            "{}, source: {}",
            e.to_string(),
            e.source().map(<dyn Error>::to_string).unwrap_or_default()
        )
    })
}

#[tauri::command]
async fn open_location(path: &str) -> Result<(), String> {
    let path = Utf8Path::new(path);
    crate::utility::show_file_location(path).map_err(|e| {
        format!(
            "{}, source: {}",
            e.to_string(),
            e.source().map(<dyn Error>::to_string).unwrap_or_default()
        )
    })
}

#[tauri::command]
async fn index(paths: Vec<String>) -> Result<(), String> {
    let data_dir = app_config::get_default_index_directory();
    let lancedbstore = LanceDBStore::new(data_dir.as_str(), 768)
        .await
        .map_err(|e| format!("Could not open lancedb store: {}, source: {}", e, 
            e.source().map(<dyn Error>::to_string).unwrap_or("".to_string())))?;
    let file_indexer = FileIndexer::with(lancedbstore);

    let utf8_paths: Vec<Utf8PathBuf> = paths.into_iter().map(|p| Utf8PathBuf::from(p)).collect();
    let unique_files = crate::utility::explore_paths(utf8_paths);

    for path in &unique_files {
        println!("Indexing file: {}", path);
        file_indexer
            .index(path)
            .await
            .map_err(|e| format!("Error while indexing files: {}, source: {}", e,
                e.source().map(<dyn Error>::to_string).unwrap_or("".to_string())))?;
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            index,
            open,
            open_location,
            preview,
            query,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub mod utility;
