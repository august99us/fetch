use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::io::{self, Write};
use std::{env, fs};
use std::path::{Path, PathBuf};
use reqwest::blocking::Client;
use serde::Deserialize;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from("bundle");
    let models_folder = out_dir.join("models");
    println!("cargo:rerun-if-changed={}/", out_dir.to_str().unwrap());

    match download_onnx_runtime(&out_dir) {
        Ok(_) => {},
        Err(e) => {
            println!("cargo:error=Failed to load ONNX Runtime files: {}", e);
        }
    }

    #[cfg(feature = "pdf")]
    match pdfium::download_pdfium_dylib(&out_dir) {
        Ok(_) => {},
        Err(e) => {
            println!("cargo:error=Failed to load PDFium files: {}", e);
        }
    }

    // disable model downloading for windows because it increases the size of the bundle
    // too much for light.exe to handle. models for windows must be packaged separately
    #[cfg(not(target_os = "windows"))]
    {
        match download_hf_model(
            "siglip2-base-patch16-512", 
            "august99us/siglip2-base-patch16-512-fetch", 
            &models_folder
        ) {
            Ok(_) => {},
            Err(e) => {
                println!("cargo:error=Failed to load siglip2 model files: {}", e);
            }
        }

        match download_hf_model(
            "embeddinggemma-300m",
            "august99us/embeddinggemma-300m-fetch",
            &models_folder
        ) {
            Ok(_) => {},
            Err(e) => {
                println!("cargo:error=Failed to load embeddinggemma model files: {}", e);
            }
        }
    }
}

#[derive(Deserialize)]
struct VersionInfoJson {
    tag_name: String,
}

#[cfg(feature = "pdf")]
mod pdfium {
    use super::*;

    // List of pdfium library files we want to extract
    #[cfg(target_os = "windows")]
    pub const PDFIUM_LIB_PATTERNS: [&str; 1] = [
        // windows patterns
        "pdfium.dll",
    ];
    #[cfg(target_os = "macos")]
    pub const PDFIUM_LIB_PATTERNS: [&str; 1] = [
        // mac patterns
        "libpdfium.dylib",
    ];
    #[cfg(target_os = "linux")]
    pub const PDFIUM_LIB_PATTERNS: [&str; 1] = [
        // linux patterns
        "libpdfium.so",
    ];

    pub fn download_pdfium_dylib(out_dir: &Path) -> Result<(), Box<dyn Error>> {
        let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
        // x86_64, aarch64, etc.
        let architecture = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

        // Check if pdfium libraries already exist
        let lib_exists = match target_os.as_str() {
            "windows" => out_dir.join("pdfium.dll").exists(),
            "macos" => out_dir.join("libpdfium.dylib").exists(),
            "linux" => out_dir.join("libpdfium.so").exists(),
            _ => {
                println!(
                    "cargo:warning=Unsupported OS: {}. Skipping pdfium prep.",
                    target_os
                );
                return Ok(());
            }
        };
        if lib_exists {
            println!("cargo:warning=Pdfium libraries already exist in bundle/, skipping download");
            return Ok(());
        }

        // Skip download if ONNX_BUILD_PATH is set (user is providing their own ONNX Runtime)
        println!("cargo:rerun-if-env-changed=PDFIUM_BUILD_PATH");
        if let Ok(pdfium_build_path) = env::var("PDFIUM_BUILD_PATH") {
            println!("cargo:warning=Using custom pdfium build from PDFIUM_BUILD_PATH");
            if let Err(e) = copy_libs_to_path_recursive(
                Path::new(&pdfium_build_path),
                out_dir,
                &PDFIUM_LIB_PATTERNS
            ) {
                println!("cargo:error=Failed to copy dylibs from PDFIUM_BUILD_PATH: {}", e);
                return Err(e);
            }
            return Ok(());
        }

        // Resolve latest version
        let client = reqwest_client()?;
        println!("cargo:rerun-if-env-changed=PDFIUM_RELEASE_VERSION_DOWNLOAD");
        // Should resolve to a string like "chromium/7520", no v prefix or anything
        let pdfium_version = match env::var("PDFIUM_RELEASE_VERSION_DOWNLOAD") {
            Ok(ver) => ver,
            Err(_) => {
                let response = client
                    .get("https://api.github.com/repos/bblanchon/pdfium-binaries/releases/latest")
                    .send()?;
                if !response.status().is_success() {
                    return Err(format!(
                        "Failed to contact github for pdfium release info: HTTP {}, Response: {}",
                        response.status(),
                        response.text().unwrap_or_default()
                    )
                    .into());
                }
                let tag = response.json::<VersionInfoJson>().unwrap().tag_name;
                if tag.chars().next().expect("tag is empty") == 'v' {
                    tag[1..].to_owned()
                } else {
                    tag
                }
            }
        };

        // Construct download URL based on version, platform and architecture
        let filename = match (target_os.as_ref(), architecture.as_ref()) {
            ("windows", "x86_64") => "pdfium-win-x64.tgz",
            ("windows", "aarch64") => "pdfium-win-arm64.tgz",
            ("macos", _) => "pdfium-mac-univ.tgz",
            ("linux", "x86_64") => "pdfium-linux-x64.tgz",
            ("linux", "aarch64") => "pdfium-linux-arm64.tgz",
            _ => return Err(format!("Unsupported platform: {}", target_os).into()),
        };

        let url = format!(
            "https://github.com/bblanchon/pdfium-binaries/releases/download/{}/{}",
            pdfium_version, filename
        );

        println!(
            "cargo:warning=Downloading pdfium version {} for {}-{}...",
            pdfium_version, target_os, architecture,
        );
        println!("cargo:warning=Downloading from: {}", url);

        // Download the file
        let response = client.get(&url).send()?;
        if !response.status().is_success() {
            return Err(format!("Failed to download: HTTP {}", response.status()).into());
        }

        let bytes = response.bytes()?;

        // Create a temporary file for the archive
        #[allow(clippy::needless_borrows_for_generic_args)] // filename is used later
        let temp_archive = PathBuf::from(env::var("OUT_DIR")?).join(&filename);
        let mut file = File::create(&temp_archive)?;
        file.write_all(&bytes)?;
        drop(file);

        println!("cargo:warning=Downloaded {} MB", bytes.len() / 1_000_000);
        println!("cargo:warning=Extracting libraries to bundle/...");

        if filename.ends_with(".zip") {
            extract_from_zip(&temp_archive, out_dir, &PDFIUM_LIB_PATTERNS)?;
        } else if filename.ends_with(".tgz") {
            extract_from_tar_gz(&temp_archive, out_dir, &PDFIUM_LIB_PATTERNS)?;
        }

        // Clean up temp file
        fs::remove_file(temp_archive)?;

        Ok(())
    }
}

#[derive(Deserialize, Debug)]
struct HfFile {
    rfilename: String,
}

#[derive(Deserialize, Debug)]
struct HfModelInfo {
    siblings: Vec<HfFile>,
}

fn download_hf_model(model_name: &str, repo_id: &str, out_folder: &Path) -> Result<(), Box<dyn Error>> {
    if !out_folder.exists() {
        fs::create_dir_all(out_folder)?;
    }

    // Create the destination directory
    let model_dir = out_folder.join(model_name);

    if model_dir.exists() {
        if model_dir.is_dir() {
            println!("cargo:warning={} files already exist in bundle/models, skipping download", model_name);
        } else {
            println!("cargo:warning={} item exists in bundle/models, but is not a folder. skipping", model_name);
        }
        return Ok(());
    }

    let client = reqwest_client()?;
    // Get the repository file list
    let api_url = format!("https://huggingface.co/api/models/{}", repo_id);
    println!("cargo:warning=Fetching model info from {}", api_url);

    let response = client.get(&api_url).send()?;
    let model_info: HfModelInfo = response.json()?;

    fs::create_dir_all(&model_dir)?;
    // Download each file
    for file in model_info.siblings {
        let file_url = format!("https://huggingface.co/{}/resolve/main/{}", repo_id, file.rfilename);
        let dest_path = model_dir.join(&file.rfilename);

        // Create parent directories if needed
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Skip if file already exists
        if dest_path.exists() {
            println!("cargo:warning=Skipping {}, already exists", file.rfilename);
            continue;
        }

        println!("cargo:warning=Downloading {} to {}", file.rfilename, dest_path.display());

        let mut response = client.get(&file_url).send()?;
        let mut dest_file = fs::File::create(&dest_path)?;
        std::io::copy(&mut response, &mut dest_file)?;
    }

    Ok(())
}

// List of onnx library files we want to extract
#[cfg(target_os = "windows")]
const ONNX_LIB_PATTERNS: [&str; 4] = [
    // windows patterns
    "onnxruntime.dll",
    "onnxruntime_providers_shared.dll",
    "onnxruntime_providers_cuda.dll",
    "onnxruntime_providers_tensorrt.dll",
];
#[cfg(target_os = "macos")]
const ONNX_LIB_PATTERNS: [&str; 2] = [
    // mac patterns
    "libonnxruntime.dylib",
    "libonnxruntime_providers_shared.dylib",
];
#[cfg(target_os = "linux")]
const ONNX_LIB_PATTERNS: [&str; 4] = [
    // linux patterns
    "libonnxruntime.so",
    "libonnxruntime_providers_shared.so",
    "libonnxruntime_providers_cuda.so",
    "libonnxruntime_providers_tensorrt.so",
];
fn download_onnx_runtime(out_dir: &Path) -> Result<(), Box<dyn Error>> {
    if !out_dir.exists() {
        fs::create_dir_all(out_dir)?;
    }

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

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    // Check if ONNX Runtime libraries already exist
    let lib_exists = match target_os.as_str() {
        "windows" => out_dir.join("onnxruntime.dll").exists(),
        "macos" => out_dir.join("libonnxruntime.dylib").exists(),
        "linux" => out_dir.join("libonnxruntime.so").exists(),
        _ => {
            println!(
                "cargo:warning=Unsupported OS: {}. Skipping ONNX Runtime prep.",
                target_os
            );
            return Ok(());
        }
    };
    if lib_exists {
        println!("cargo:warning=ONNX Runtime libraries already exist in bundle/, skipping download");
        return Ok(());
    }

    println!("cargo:rerun-if-env-changed=ONNX_BUILD_PATH");
    // Skip download if ONNX_BUILD_PATH is set (user is providing their own ONNX Runtime)
    if let Ok(onnx_build_path) = env::var("ONNX_BUILD_PATH") {
        println!("cargo:warning=Using custom ONNX Runtime build from ONNX_BUILD_PATH");
        if let Err(e) = copy_libs_to_path_recursive(
            Path::new(&onnx_build_path),
            out_dir,
            &ONNX_LIB_PATTERNS
        ) {
            println!("cargo:error=Failed to copy dylibs from ONNX_BUILD_PATH: {}", e);
            return Err(e);
        }
        return Ok(());
    }

    // Download and extract ONNX Runtime
    if let Err(e) = download_and_extract_onnx(variant, &target_os, out_dir) {
        println!("cargo:error=Failed to download ONNX Runtime: {}", e);
        return Err(e);
    }

    println!(
        "cargo:warning=ONNX Runtime {} libraries downloaded successfully",
        variant
    );

    Ok(())
}

fn download_and_extract_onnx(
    variant: &str,
    target_os: &str,
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let client = reqwest_client()?;

    println!("cargo:rerun-if-env-changed=ONNX_RELEASE_VERSION_DOWNLOAD");
    let onnx_version = match env::var("ONNX_RELEASE_VERSION_DOWNLOAD") {
        Ok(ver) => ver,
        Err(_) => {
            let response = client
                .get("https://api.github.com/repos/microsoft/onnxruntime/releases/latest")
                .send()?;
            if !response.status().is_success() {
                return Err(format!(
                    "Failed to contact github for onnx release info: HTTP {}, Response: {}",
                    response.status(),
                    response.text().unwrap_or_default()
                )
                .into());
            }
            let tag = response.json::<VersionInfoJson>().unwrap().tag_name;
            if tag.chars().next().expect("tag is empty") == 'v' {
                tag[1..].to_owned()
            } else {
                tag
            }
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
        }
        ("macos", _) => format!("onnxruntime-osx-universal2-{}.tgz", onnx_version),
        ("linux", "gpu") => format!("onnxruntime-linux-x64-gpu-{}.tgz", onnx_version),
        ("linux", _) => format!("onnxruntime-linux-x64-{}.tgz", onnx_version),
        _ => return Err(format!("Unsupported platform: {}", target_os).into()),
    };

    let url = format!(
        "https://github.com/microsoft/onnxruntime/releases/download/v{}/{}",
        onnx_version, filename
    );

    println!(
        "cargo:warning=Downloading ONNX Runtime {} variant version {} for {}...",
        variant, onnx_version, target_os
    );
    println!("cargo:warning=Downloading from: {}", url);

    // Download the file
    let response = client.get(&url).send()?;
    if !response.status().is_success() {
        return Err(format!("Failed to download: HTTP {}", response.status()).into());
    }

    let bytes = response.bytes()?;

    // Create a temporary file for the archive
    #[allow(clippy::needless_borrows_for_generic_args)] // filename is used later
    let temp_archive = PathBuf::from(env::var("OUT_DIR")?).join(&filename);
    let mut file = File::create(&temp_archive)?;
    file.write_all(&bytes)?;
    drop(file);

    println!("cargo:warning=Downloaded {} MB", bytes.len() / 1_000_000);
    println!("cargo:warning=Extracting libraries to bundle/...");

    if filename.ends_with(".zip") {
        extract_from_zip(&temp_archive, output_dir, &ONNX_LIB_PATTERNS)?;
    } else if filename.ends_with(".tgz") {
        extract_from_tar_gz(&temp_archive, output_dir, &ONNX_LIB_PATTERNS)?;
    }

    // Clean up temp file
    fs::remove_file(temp_archive)?;

    Ok(())
}

fn copy_libs_to_path_recursive(
    source_path: &Path,
    output_dir: &Path,
    lib_patterns: &[&str],
) -> Result<(), Box<dyn Error>> {
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut queue = vec![source_path.to_path_buf()];
    while let Some(path) = queue.pop() {
        if seen.contains(&path) {
            eprintln!("Warning: Circled back to folder that was already seen before. Maybe there is a symlink creating a circular 
                directory structure somewhere? Folder: {:?}", path);
            continue;
        }

        if path.is_file() {
            let filename = path
                .file_name()
                .expect("File should have name")
                .to_str()
                .expect("Could not convert OsStr to str");
            if lib_patterns
                .iter()
                .any(|pattern| filename.ends_with(pattern))
            {
                let output_path = output_dir.join(filename);
                fs::copy(&path, output_path)?;
            }
        } else if path.is_dir() {
            for entry_result in path
                .read_dir()
                .unwrap_or_else(|_| panic!("failed reading directory: {:?}", path))
            {
                match entry_result {
                    Ok(entry) => {
                        queue.push(entry.path());
                    }
                    Err(e) => panic!("Issue reading directory entry: {e:?}"),
                }
            }
        } else {
            println!(
                "Warning: path is neither a file nor a directory, ignoring: {:?}",
                path
            );
        }
        seen.insert(path.clone());
    }

    Ok(())
}

fn extract_from_zip(zip_path: &Path, output_dir: &Path, lib_patterns: &[&str]) -> Result<(), Box<dyn Error>> {
    use zip::ZipArchive;
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let file_name = file.name();

        // Check if this is a library we want
        let should_extract = lib_patterns
            .iter()
            .any(|pattern| file_name.ends_with(pattern));

        if should_extract {
            let file_name_only = Path::new(file_name).file_name().unwrap();
            let output_path = output_dir.join(file_name_only);

            println!(
                "cargo:warning=Extracting: {}",
                file_name_only.to_string_lossy()
            );

            let mut outfile = File::create(&output_path)?;
            io::copy(&mut file, &mut outfile)?;
            drop(outfile);
        }
    }

    Ok(())
}

fn extract_from_tar_gz(tar_gz_path: &Path, output_dir: &Path, lib_patterns: &[&str]) -> Result<(), Box<dyn Error>> {
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
        let should_extract = lib_patterns
            .iter()
            .any(|pattern| file_name == *pattern || file_name.starts_with(pattern));

        if should_extract {
            let output_path = output_dir.join(file_name);

            println!("cargo:warning=Extracting: {}", file_name);

            entry.unpack(output_path)?;
        }
    }

    Ok(())
}

fn reqwest_client() -> Result<Client, Box<dyn Error>> {
    Ok(Client::builder()
        .user_agent(format!(
            "sparrow/fetch-core/build-agent/{}",
            env!("CARGO_PKG_VERSION")
        ))
        .build()?)
}
