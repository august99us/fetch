use std::fs;
use std::path::Path;
use tauri_build::Attributes;

fn main() {
    // Build Tauri app
    tauri_build::try_build(Attributes::new()).unwrap_or_else(|e| panic!("tauri error: {:?}", e));
}