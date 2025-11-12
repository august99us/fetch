use std::fs;
use std::path::PathBuf;
use tauri_build::Attributes;

fn main() {
    // TEMPORARY: Copy fetch-cli binaries to bundle folder
    copy_cli_binaries();

    // Now build Tauri app
    tauri_build::try_build(Attributes::new())
        .unwrap_or_else(|e| panic!("tauri error: {:?}", e));
}

fn copy_cli_binaries() {
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let target_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(&profile);
    let bundle_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../bundle");

    // Create bundle directory if it doesn't exist
    fs::create_dir_all(&bundle_dir)
        .expect("Failed to create bundle directory");

    // Build fetch-cli binaries first
    println!("Building fetch-cli with profile: {}", profile);
    let status = std::process::Command::new("cargo")
        .args(&["build", "-p", "fetch-cli", "--profile", &profile])
        .status()
        .expect("Failed to execute cargo build for fetch-cli");

    if !status.success() {
        panic!("Failed to build fetch-cli package");
    }

    let binaries = [
        "fetch-index",
        "fetch-query",
        "fetch-query-by-file",
        "fetch-drop",
        "fetch-daemon",
    ];

    for binary in &binaries {
        let exe_name = if cfg!(target_os = "windows") {
            format!("{}.exe", binary)
        } else {
            binary.to_string()
        };

        let src = target_dir.join(&exe_name);
        let dst = bundle_dir.join(&exe_name);

        if src.exists() {
            println!("cargo:rerun-if-changed={}", src.display());
            fs::copy(&src, &dst)
                .unwrap_or_else(|e| panic!("Failed to copy {} to bundle: {}", binary, e));
            println!("Copied {} to bundle", binary);
        } else {
            println!("cargo:warning=Binary {} not found at {}, skipping", binary, src.display());
        }
    }
}