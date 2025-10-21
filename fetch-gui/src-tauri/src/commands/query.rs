use fetch_core::file_index::query_files::QueryFiles;
use serde::Serialize;

use crate::utility::get_file_indexer;

#[derive(Debug, Serialize)]
pub struct QueryResult {
    pub name: String,
    pub path: String,
    pub score: f32,
}

#[tauri::command]
pub async fn query(query: &str, page: u32) -> Result<Vec<QueryResult>, String> {
    let file_indexer = get_file_indexer().await?;

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
