use core::hash;
use std::collections::HashSet;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::process::Stdio;

use camino::Utf8Path;
use camino::Utf8PathBuf;

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

pub fn open_file_with_default_app(path: &Utf8Path) -> Result<(), anyhow::Error> {
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

/// Expands the paths given, returning all files and files found while exploring directories.
/// Ignores non-existant paths
pub fn explore_paths(paths: Vec<Utf8PathBuf>) -> Vec<Utf8PathBuf> {
    let mut seen: HashSet<Utf8PathBuf> = HashSet::new();
    let mut files: HashSet<Utf8PathBuf> = HashSet::new();
    let mut queue = paths;
    while let Some(path) = queue.pop() {
        if seen.contains(&path) {
            eprintln!("Warning: Circled back to folder that was already seen before. Maybe there is a symlink creating a circular 
                directory structure somewhere? Folder: {}", path.to_string());
                continue;
        }
        
        if path.is_file() {
            files.insert(path.clone());
        } else if path.is_dir() {
            for entry_result in path.read_dir()
                .unwrap_or_else(|_| panic!("failed reading directory: {}", path.to_string())) {
                match entry_result {
                    Ok(entry) => {
                        let convert_result = Utf8PathBuf::try_from(entry.path());
                        match convert_result {
                            Err(e) => {
                                eprintln!("Warning: could not convert pathbuf to utf8pathbuf, ignoring path: {}, error: {e:?}",
                                    entry.path().to_string_lossy());
                                continue;
                            },
                            Ok(entry_path) => {
                                queue.push(entry_path);
                            },
                        }
                    },
                    Err(e) => panic!("Issue reading directory entry: {e:?}"),
                }
            }
        } else {
            println!("Warning: path is neither a file nor a directory, ignoring: {}", path.to_string());
        }
        seen.insert(path);
    }
    files.into_iter().collect()
}
