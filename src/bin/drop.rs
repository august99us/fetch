use std::error::Error;

use camino::Utf8PathBuf;
use clap::Parser;
use fetch::vector_store::lancedb_store::LanceDBStore;

#[derive(Parser, Debug)]
#[command(name = "fetch-drop")]
#[command(author = "August Sun, august99us@gmail.com")]
#[command(version = "0.1")]
#[command(about = "drops entire database (development use)", long_about = None)]
struct Args {
    // Does nothing currently
    #[arg(short, long)]
    verbose: bool,
    // Directory where index is stored
    #[arg(long)]
    data_directory: Utf8PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    LanceDBStore::drop(args.data_directory.as_str()).await?;

    println!("Completed clearing and regenerating lancedb database at {}", &args.data_directory);

    Ok(())
}