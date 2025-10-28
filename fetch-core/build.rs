use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use reqwest::blocking::Client;
use serde::Deserialize;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let models_folder = PathBuf::from("bundle/models");
    println!("cargo:rerun-if-changed=bundle/");

    match download_hf_model(
        "siglip2-base-patch16-512", 
        "august99us/siglip2-base-patch16-512-fetch", 
        &models_folder
    ) {
        Ok(_) => println!("cargo:warning=Successfully loaded model files"),
        Err(e) => {
            println!("cargo:error=Failed to load model files: {}", e);
        }
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
        fs::create_dir_all(&out_folder)?;
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

fn reqwest_client() -> Result<Client, Box<dyn Error>> {
    Ok(Client::builder()
        .user_agent(format!(
            "sparrow/fetch-core/build-agent/{}",
            env!("CARGO_PKG_VERSION")
        ))
        .build()?)
}
