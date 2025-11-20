use std::fs;
use std::path::Path;
use tauri_build::Attributes;

fn main() {
    // Sync version from Cargo.toml to JSON config files
    sync_version_to_json_files();

    // Build Tauri app
    tauri_build::try_build(Attributes::new()).unwrap_or_else(|e| panic!("tauri error: {:?}", e));
}

/// Synchronizes the version from Cargo.toml to tauri.conf.json and package.json.
/// This ensures all version numbers stay in sync by reading from the workspace package version
/// and updating the JSON files during the build process.
///
/// Rerun behavior: This function automatically runs whenever the workspace Cargo.toml changes
/// (where the version is defined). If the version property is manually touched in the json files,
/// then these versions may become unsynced.
fn sync_version_to_json_files() {
    let version = env!("CARGO_PKG_VERSION");

    // Update tauri.conf.json
    let tauri_conf_path = Path::new("tauri.conf.json");
    if tauri_conf_path.exists() {
        let contents = fs::read_to_string(tauri_conf_path)
            .expect("Failed to read tauri.conf.json");

        let mut json: serde_json::Value = serde_json::from_str(&contents)
            .expect("Failed to parse tauri.conf.json");

        let json_version = json["version"].as_str()
            .expect("tauri.conf.json should have version string");

        if version != json_version {
            json["version"] = serde_json::json!(version);

            let updated = serde_json::to_string_pretty(&json)
                .expect("Failed to serialize tauri.conf.json");

            fs::write(tauri_conf_path, updated)
                .expect("Failed to write tauri.conf.json");
        }
    }

    // Update package.json (in parent directory)
    let package_json_path = Path::new("../package.json");
    if package_json_path.exists() {
        let contents = fs::read_to_string(package_json_path)
            .expect("Failed to read package.json");

        let mut json: serde_json::Value = serde_json::from_str(&contents)
            .expect("Failed to parse package.json");

        let json_version = json["version"].as_str()
            .expect("tauri.conf.json should have version string");

        if version != json_version {
            json["version"] = serde_json::json!(version);

            let updated = serde_json::to_string_pretty(&json)
                .expect("Failed to serialize package.json");

            fs::write(package_json_path, updated)
                .expect("Failed to write package.json");
        }
    }
}