use std::error::Error;
use std::process::{Command, Stdio};

use camino::Utf8Path;

#[tauri::command]
pub async fn open(path: &str) -> Result<(), String> {
    let path = Utf8Path::new(path);
    open_file_with_default_app(path).map_err(|e| {
        format!(
            "{}, source: {}",
            e.to_string(),
            e.source().map(<dyn Error>::to_string).unwrap_or_default()
        )
    })
}

// Private functions

fn open_file_with_default_app(path: &Utf8Path) -> Result<(), Box<dyn Error>> {
    #[cfg(target_os = "windows")]
    Command::new("cmd")
        .args(["/c", "start", "", &path.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    #[cfg(target_os = "macos")]
    Command::new("open")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    #[cfg(target_os = "linux")]
    Command::new("xdg-open")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(())
}
