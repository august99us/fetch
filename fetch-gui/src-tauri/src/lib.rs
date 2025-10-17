// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
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