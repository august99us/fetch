use camino::Utf8Path;
use fetch_core::previewable::PossiblyPreviewable;

#[tauri::command]
pub async fn preview(path: &str) -> Result<Option<String>, String> {
    let path = Utf8Path::new(path);
    match path.preview().await {
        Ok(Some(previewed_file)) => Ok(Some(previewed_file.preview_path.to_string())),
        Ok(None) => Ok(None),
        Err(e) => Err(format!("Error while getting preview: {}", e.to_string())),
    }
}
