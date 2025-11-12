use std::fs;
use std::path::PathBuf;
use tauri_build::Attributes;

fn main() {
    // Now build Tauri app
    tauri_build::try_build(Attributes::new()).unwrap_or_else(|e| panic!("tauri error: {:?}", e));
}