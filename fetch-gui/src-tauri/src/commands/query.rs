use std::error::Error;

use fetch_core::files::query::QueryFiles;
use serde::Serialize;

use crate::utility::get_file_queryer;

#[derive(Debug, Serialize)]
pub struct FileQueryingResult {
    pub results_len: u32,
    pub changed_results: Vec<QueryResult>,
    pub cursor_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QueryResult {
    pub name: String,
    pub path: String,
    pub old_rank: Option<u32>,
    pub rank: u32,
    pub score: f32,
}

#[tauri::command]
pub async fn query(query: &str, cursor_id: Option<&str>) -> Result<FileQueryingResult, String> {
    let file_queryer = get_file_queryer().await?;

    file_queryer
        .query_n(query, 100, cursor_id)
        .await
        .map(|result| {
            FileQueryingResult {
                results_len: result.results_len,
                changed_results: result.changed_results.into_iter()
                    .map(|query_result| QueryResult {
                        name: query_result
                            .path
                            .file_name()
                            .expect("Result path should have a name")
                            .to_string(),
                        path: query_result.path.to_string(),
                        old_rank: query_result.old_rank,
                        rank: query_result.rank,
                        score: query_result.score,
                    })
                    .collect(),
                cursor_id: result.cursor_id,
            }
        })
        .map_err(|e| format!("{}, source: {:?}", e, e.source()))
}
