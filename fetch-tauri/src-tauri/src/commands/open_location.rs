use std::error::Error;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};

use camino::Utf8Path;

#[tauri::command]
pub async fn open_location(path: &str) -> Result<(), String> {
    let path = Utf8Path::new(path);
    show_file_location(path).map_err(|e| {
        format!(
            "{}, source: {}",
            e.to_string(),
            e.source().map(<dyn Error>::to_string).unwrap_or_default()
        )
    })
}

// Private functions

fn show_file_location(path: &Utf8Path) -> Result<(), Box<dyn Error>> {
    #[cfg(target_os = "windows")]
    let res = Command::new("explorer.exe")
        .raw_arg(format!("/select,{}", path.to_string()))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    #[cfg(target_os = "macos")]
    let res = Command::new("open")
        .arg("-R")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    #[cfg(target_os = "linux")]
    // TODO: use dbus-send?
    Command::new("nautilus")
        .arg("--select")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(())
}