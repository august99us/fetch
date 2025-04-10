use std::{error::Error, path::{self, PathBuf}};

use camino::Utf8PathBuf;
use clap::Parser;
use embed_anything::embeddings::embed::Embedder;
use fetch::{file_index::{index_files::{FileIndexing, IndexFiles}, FileIndexer}, vector_store::lancedb_store::LanceDBStore};
use normalize_path::NormalizePath;

#[derive(Parser, Debug)]
#[command(name = "fetch-indexer")]
#[command(author = "August Sun, august99us@gmail.com")]
#[command(version = "0.1")]
#[command(about = "indexes things semantically", long_about = None)]
struct Args {
    #[arg(short, long)]
    verbose: bool,
    data_directory: Utf8PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    LanceDBStore::drop(args.data_directory.as_str()).await?;

    println!("Completed clearing and regenerating lancedb database at {}", &args.data_directory);

    Ok(())
}