use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Deserialize;

fn main() {
    // Download ONNX Runtime BEFORE calling tauri_build
    download_onnx_runtime();

    // Now build Tauri app
    tauri_build::build();
}

fn download_onnx_runtime() {
    // Determine which ONNX Runtime variant to download
    let variant = if cfg!(feature = "cuda") {
        "gpu" // GPU variant includes CUDA support
    } else if cfg!(feature = "qnn") {
        // QNN requires custom build, error out for now
        panic!("QNN feature requires building ONNX Runtime from source. Not yet supported in automated builds.");
    } else {
        "cpu" // Default CPU-only variant
    };

    println!("cargo:rerun-if-changed=build.rs");

    let bundle_dir = PathBuf::from("bundle/onnx-libs");
    fs::create_dir_all(&bundle_dir).unwrap_or_else(|_| panic!("Could not create bundle/onnx-libs dir"));

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    // Check if ONNX Runtime libraries already exist
    let lib_exists = match target_os.as_str() {
        "windows" => bundle_dir.join("onnxruntime.dll").exists(),
        "macos" => bundle_dir.join("libonnxruntime.dylib").exists(),
        "linux" => bundle_dir.join("libonnxruntime.so").exists(),
        _ => {
            println!("cargo:warning=Unsupported OS: {}. Skipping ONNX Runtime prep.", target_os);
            return;
        }
    };
    if lib_exists {
        println!("cargo:warning=ONNX Runtime libraries already exist in bundle/onnx-libs/, skipping download");
        return;
    }

    println!("cargo:rerun-if-env-changed=ONNX_BUILD_PATH");
    // Skip download if ONNX_BUILD_PATH is set (user is providing their own ONNX Runtime)
    if let Ok(onnx_build_path) = env::var("ONNX_BUILD_PATH") {
        println!("cargo:warning=Using custom ONNX Runtime build from ONNX_BUILD_PATH");
        if let Err(e) = copy_libs_to_path_recursive(Path::new(&onnx_build_path), &bundle_dir) {
            panic!("Failed to copy dylibs from ONNX_BUILD_PATH: {}", e);
        }
        return;
    }

    // Download and extract ONNX Runtime
    if let Err(e) = download_and_extract_onnx(variant, &target_os, &bundle_dir) {
        panic!("Failed to download ONNX Runtime: {}", e);
    }

    println!("cargo:warning=ONNX Runtime {} libraries downloaded successfully", variant);
}

#[derive(Deserialize)]
struct OnnxInfoJson {
    tag_name: String,
}

fn download_and_extract_onnx(variant: &str, target_os: &str, output_dir: &Path) -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-env-changed=ONNX_RELEASE_VERSION_DOWNLOAD");
    let onnx_version = match env::var("ONNX_RELEASE_VERSION_DOWNLOAD") {
        Ok(ver) => ver,
        Err(_) => {
            let response = reqwest::blocking::get("https://api.github.com/repos/microsoft/onnxruntime/releases/latest")?;
            if !response.status().is_success() {
                return Err(format!("Failed to contact github for onnx release info: HTTP {}", response.status()).into());
            }
            response.json::<OnnxInfoJson>().unwrap().tag_name
        }
    };

    // Construct download URL based on platform and variant
    let filename = match (target_os, variant) {
        ("windows", "gpu") => format!("onnxruntime-win-x64-gpu-{}.zip", onnx_version),
        ("windows", _) => format!("onnxruntime-win-x64-{}.zip", onnx_version),
        ("macos", "gpu") => {
            // macOS doesn't have official GPU builds, fall back to CPU
            println!("cargo:warning=macOS GPU build not available, using CPU variant");
            format!("onnxruntime-osx-universal2-{}.tgz", onnx_version)
        },
        ("macos", _) => format!("onnxruntime-osx-universal2-{}.tgz", onnx_version),
        ("linux", "gpu") => format!("onnxruntime-linux-x64-gpu-{}.tgz", onnx_version),
        ("linux", _) => format!("onnxruntime-linux-x64-{}.tgz", onnx_version),
        _ => return Err(format!("Unsupported platform: {}", target_os).into()),
    };

    let url = format!(
        "https://github.com/microsoft/onnxruntime/releases/download/v{}/{}",
        onnx_version, filename
    );

    println!("cargo:warning=Downloading ONNX Runtime {} variant version {} for {}...", variant, onnx_version, target_os);
    println!("cargo:warning=Downloading from: {}", url);

    // Download the file
    let response = reqwest::blocking::get(&url)?;
    if !response.status().is_success() {
        return Err(format!("Failed to download: HTTP {}", response.status()).into());
    }

    let bytes = response.bytes()?;

    // Create a temporary file for the archive
    let temp_archive = PathBuf::from(env::var("OUT_DIR")?).join(&filename);
    let mut file = File::create(&temp_archive)?;
    file.write_all(&bytes)?;
    drop(file);

    println!("cargo:warning=Downloaded {} MB", bytes.len() / 1_000_000);
    println!("cargo:warning=Extracting libraries to bundle/onnx-libs/...");

    if filename.ends_with(".zip") {
        extract_from_zip(&temp_archive, output_dir)?;
    } else if filename.ends_with(".tgz") {
        extract_from_tar_gz(&temp_archive, output_dir)?;
    }

    // Clean up temp file
    fs::remove_file(temp_archive)?;

    Ok(())
}

// List of library files we want to extract
#[cfg(target_os = "windows")]
const LIB_PATTERNS: [&str; 4] = [
    // windows patterns
    "onnxruntime.dll",
    "onnxruntime_providers_shared.dll",
    "onnxruntime_providers_cuda.dll",
    "onnxruntime_providers_tensorrt.dll",
];
#[cfg(target_os = "macos")]
const LIB_PATTERNS: [&str; 2] = [
    // windows patterns
    "libonnxruntime.dylib",
    "libonnxruntime_providers_shared.dylib",
];
#[cfg(target_os = "linux")]
const LIB_PATTERNS: [&str; 4] = [
    // linux patterns
    "libonnxruntime.so",
    "libonnxruntime_providers_shared.so",
    "libonnxruntime_providers_cuda.so",
    "libonnxruntime_providers_tensorrt.so",
];
fn copy_libs_to_path_recursive(source_path: &Path, output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut queue = vec![source_path.to_path_buf()];
    while let Some(path) = queue.pop() {
        if seen.contains(&path) {
            eprintln!("Warning: Circled back to folder that was already seen before. Maybe there is a symlink creating a circular 
                directory structure somewhere? Folder: {:?}", path);
                continue;
        }
        
        if path.is_file() {
            let filename = path.file_name().expect("File should have name")
                .to_str().expect("Could not convert OsStr to str");
            if LIB_PATTERNS.iter().any(|pattern| filename.ends_with(pattern)) {
                let output_path = output_dir.join(filename);
                fs::copy(&path, output_path)?;
            }
        } else if path.is_dir() {
            for entry_result in path.read_dir()
                .unwrap_or_else(|_| panic!("failed reading directory: {:?}", path)) {
                match entry_result {
                    Ok(entry) => {
                        queue.push(entry.path());
                    },
                    Err(e) => panic!("Issue reading directory entry: {e:?}"),
                }
            }
        } else {
            println!("Warning: path is neither a file nor a directory, ignoring: {:?}", path);
        }
        seen.insert(path.clone());
    }

    Ok(())
}

fn extract_from_zip(zip_path: &Path, output_dir: &Path) -> Result<(), Box<dyn Error>> {
    use zip::ZipArchive;
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_name = file.name();

        // Check if this is a library we want
        let should_extract = LIB_PATTERNS.iter().any(|pattern| {
            file_name.ends_with(pattern)
        });

        if should_extract {
            let file_name_only = Path::new(file_name).file_name().unwrap();
            let output_path = output_dir.join(file_name_only);

            println!("cargo:warning=Extracting: {}", file_name_only.to_string_lossy());

            let mut outfile = File::create(&output_path)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}

fn extract_from_tar_gz(tar_gz_path: &Path, output_dir: &Path) -> Result<(), Box<dyn Error>> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let file = File::open(tar_gz_path)?;
    let decompressor = GzDecoder::new(file);
    let mut archive = Archive::new(decompressor);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        // Check if this is a library we want
        let should_extract = LIB_PATTERNS.iter().any(|pattern| {
            file_name == *pattern || file_name.starts_with(pattern)
        });

        if should_extract {
            let output_path = output_dir.join(file_name);

            println!("cargo:warning=Extracting: {}", file_name);

            let mut outfile = File::create(&output_path)?;
            io::copy(&mut entry, &mut outfile)?;
        }
    }

    Ok(())
}