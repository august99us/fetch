use std::{error::Error, path::PathBuf, sync::Arc};

use camino::Utf8PathBuf;
use chrono::Utc;
use clap::Parser;
use fetch_core::{app_config, index::{ChunkFile, ChunkType, embedding::siglip2::{self, Siglip2EmbeddedChunkFile}}, init_ort, store::{QueryByVector, lancedb::LanceDBStore}};
use serde_json::Map;

#[derive(Parser, Debug)]
#[command(name = "fetch-query-by-file")]
#[command(author = "August Sun, august99us@gmail.com")]
#[command(version = "0.0.3")]
#[command(about = "queries semantic file index with a query file", long_about = None)]
struct Args {
    /// Verbose mode
    #[arg(short, long)]
    verbose: bool,
    /// String to query files with
    query: PathBuf,
    /// The number of file results to return, default 20
    #[arg(short = 'n', long, default_value_t = 20)]
    num_results: u32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    init_ort(None)?;
    env_logger::init();

    let data_dir = app_config::get_default_index_directory();

    // Create the image index store
    let siglip_store: Arc<LanceDBStore<Siglip2EmbeddedChunkFile>> = Arc::new(LanceDBStore::local_full(
        data_dir.as_str(),
        "siglip2_chunkfile".to_owned()
    ).await
    .unwrap_or_else(|e| 
        panic!("Could not open lancedb store for image index with data dir: {}. Error: {e:?}",
        data_dir.as_str())));
    
    let temp_chunkfile = ChunkFile {
        original_file: Utf8PathBuf::default(),
        chunk_channel: "".to_owned(),
        chunk_sequence_id: 0.0,
        chunkfile: Utf8PathBuf::from_path_buf(args.query).unwrap(),
        chunk_type: ChunkType::Image,
        chunk_length: 1.0,
        original_file_creation_date: Utc::now(),
        original_file_modified_date: Utc::now(),
        original_file_size: 1,
        original_file_tags: Map::new(),
    };

    let vec = siglip2::embed_chunk(temp_chunkfile).await?.embedding;

    let results = siglip_store.query_vector_n(vec, 30, 0).await?;

    if results.is_empty() {
        println!("No results!");
    } else {
        println!("Results ({}):", results.len());
        for (i, result) in results.iter().enumerate() {
            println!(
                "{}: {} (distance: {:.2})",
                i + 1,
                result.result.chunkfile.original_file,
                result.distance
            );
        }
    }

    Ok(())
}