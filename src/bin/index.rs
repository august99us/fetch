use std::{error::Error, path::{self, PathBuf}};

use camino::Utf8PathBuf;
use clap::Parser;
use embed_anything::embeddings::embed::Embedder;
use fetch::{file_index::{index_files::{FileIndexing, IndexFiles}, FileIndexer}, vector_store::lancedb_store::LanceDBStore};
use normalize_path::NormalizePath;

#[derive(Parser, Debug)]
#[command(name = "fetch-index")]
#[command(author = "August Sun, august99us@gmail.com")]
#[command(version = "0.1")]
#[command(about = "indexes things semantically", long_about = None)]
struct Args {
    #[arg(short, long)]
    verbose: bool,
    file_paths: Vec<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let mut file_paths = args.file_paths.into_iter() // consume vec and iter
        .map(|pb| path::absolute(pb) // convert path to absolute path if relative
            .map(|ap| ap.normalize())) // normalize the absolute path
        .collect::<Result<Vec<PathBuf>, std::io::Error>>() // collect
        .expect("Could not get current directory to convert path to absolute path"); // propagate error
        // Technically, the path::absolute() function can error on two things: 1) can't get current error, or
        // 2) path is empty. (https://doc.rust-lang.org/stable/std/path/fn.absolute.html) We don't have to worry about
        // the "path is empty" situation because clap will not fill the args with a value if the provided argument
        // is empty.
    file_paths.sort();
    file_paths.dedup();
    let file_paths: Vec<Utf8PathBuf> = file_paths.into_iter().map(|pb| Utf8PathBuf::from_path_buf(pb)) // Convert to Utf8PathBuf
        .collect::<Result<Vec<Utf8PathBuf>, PathBuf>>() // collect results
        .unwrap_or_else(|e| panic!("Error verifying utf8 validity of path: {:?}", e));

    // TODO: unwrap error handling
    let embedder = Embedder::from_pretrained_hf("clip", "openai/clip-vit-base-patch32", None).unwrap();

    let lancedbstore = LanceDBStore::new("./data_dir", 512).await?;
    // TODO: unwrap error handling
    let file_indexer = FileIndexer::with(embedder, lancedbstore).unwrap();

    let results = file_indexer.index_multiple(file_paths.iter().map(AsRef::as_ref).collect()).await;

    for result in results {
        match result {
            Ok(FileIndexing::Result { path, r#type: FileIndexing::ResultType::Indexed }) => println!("File {path:?} \
                successfully indexed"),
            Ok(FileIndexing::Result { path, r#type: FileIndexing::ResultType::Cleared  }) => println!("File {path:?} \
                not found or could not be previewed, successfully cleared from index"),
            Err(e) => println!("Error while processing file with path {:?}: {:?}", e.path, e.source()),
        }
    }

    Ok(())
}