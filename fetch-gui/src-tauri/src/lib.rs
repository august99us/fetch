// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use camino::Utf8PathBuf;
use fetch_core::{init_indexing, init_ort, init_querying};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Get the resource directory where models are bundled
            let resource_dir = Utf8PathBuf::try_from(app.path().resource_dir()
                .expect("Failed to get resource directory"))
                .expect("Resource directory path is not valid UTF-8");

            // Initialize ort first
            init_ort(Some(&resource_dir)).expect("Failed initializing ort");

            // Convert to Utf8PathBuf and set as the base model directory
            let models_dir = resource_dir.join("models");

            // Set the resource directory with the first init call
            init_indexing(Some(&models_dir));
            // Second call doesn't need to set it again since fetch-core defines this as static setup
            init_querying(None);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::commands::index::index,
            crate::commands::open::open,
            crate::commands::open_location::open_location,
            crate::commands::preview::preview,
            crate::commands::query::query,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub mod utility;
pub mod commands;