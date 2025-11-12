use std::{collections::HashSet, error::Error};

use camino::Utf8PathBuf;
use chrono::Utc;
use fetch_core::files::index::IndexFiles;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::utility::get_file_indexer;

const PROGRESS_EVENT_IDENTIFIER: &str = "index_progress";
#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub current: f32,
    pub total: f32,
}

const LOG_EVENT_IDENTIFIER: &str = "index_log";
#[derive(Debug, Clone, Serialize)]
pub struct Log {
    pub message: String,
}

#[tauri::command]
pub async fn index(app: AppHandle, paths: Vec<String>) -> Result<(), String> {
    let file_indexer = get_file_indexer().await?;

    let utf8_paths: Vec<Utf8PathBuf> = paths.into_iter().map(Utf8PathBuf::from).collect();
    let unique_files = explore_paths(utf8_paths);

    let num_files = unique_files.len();
    app.emit_to(
        "full",
        PROGRESS_EVENT_IDENTIFIER,
        Progress {
            current: 0.0,
            total: num_files as f32,
        },
    )
    .unwrap_or_else(|e: tauri::Error| {
        eprintln!("Could not emit progress event: {}", e)
    });

    for (i, path) in unique_files.iter().map(Utf8PathBuf::as_path).enumerate() {
        app.emit_to(
            "full",
            LOG_EVENT_IDENTIFIER,
            Log {
                message: format!("Indexing file: {}", path),
            },
        )
        .unwrap_or_else(|e: tauri::Error| eprintln!("Could not emit log event: {}", e));

        file_indexer.index(path, Some(Utc::now())).await.map_err(|e| {
            format!(
                "Error while indexing files: {}, source: {}",
                e,
                e.source()
                    .map(<dyn Error>::to_string)
                    .unwrap_or("".to_string())
            )
        })?;

        app.emit_to(
            "full",
            PROGRESS_EVENT_IDENTIFIER,
            Progress {
                current: i as f32 + 1.0,
                total: num_files as f32,
            },
        )
        .unwrap_or_else(|e: tauri::Error| {
            eprintln!("Could not emit progress event: {}", e)
        });
    }

    app.emit_to(
        "full",
        LOG_EVENT_IDENTIFIER,
        Log {
            message: "All done! Goodbye.".to_string(),
        },
    )
    .unwrap_or_else(|e: tauri::Error| eprintln!("Could not emit log event: {}", e));

    Ok(())
}

// Private functions

/// Expands the paths given, returning all files and files found while exploring directories.
/// Ignores non-existant paths
fn explore_paths(paths: Vec<Utf8PathBuf>) -> Vec<Utf8PathBuf> {
    let mut seen: HashSet<Utf8PathBuf> = HashSet::new();
    let mut files: HashSet<Utf8PathBuf> = HashSet::new();
    let mut queue = paths;
    while let Some(path) = queue.pop() {
        if seen.contains(&path) {
            eprintln!("Warning: Circled back to folder that was already seen before. Maybe there is a symlink creating a circular
                directory structure somewhere? Folder: {}", path);
            continue;
        }

        if path.is_file() {
            files.insert(path.clone());
        } else if path.is_dir() {
            for entry_result in path
                .read_dir()
                .unwrap_or_else(|_| panic!("failed reading directory: {}", path))
            {
                match entry_result {
                    Ok(entry) => {
                        let convert_result = Utf8PathBuf::try_from(entry.path());
                        match convert_result {
                            Err(e) => {
                                eprintln!("Warning: could not convert pathbuf to utf8pathbuf, ignoring path: {}, error: {e:?}",
                                    entry.path().to_string_lossy());
                                continue;
                            }
                            Ok(entry_path) => {
                                queue.push(entry_path);
                            }
                        }
                    }
                    Err(e) => panic!("Issue reading directory entry: {e:?}"),
                }
            }
        } else {
            println!(
                "Warning: path is neither a file nor a directory, ignoring: {}",
                path
            );
        }
        seen.insert(path);
    }
    files.into_iter().collect()
}
