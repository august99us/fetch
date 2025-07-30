#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::process::Stdio;

use camino::Utf8Path;

pub fn show_file_location(path: &Utf8Path) -> Result<(), anyhow::Error> {
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
        .arg(path.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    #[cfg(target_os = "linux")]
    // TODO: use dbus-send?
    Command::new("nautilus")
        .arg("--select")
        .arg(path.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(())
}