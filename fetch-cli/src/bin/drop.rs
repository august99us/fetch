use std::error::Error;

use camino::Utf8PathBuf;
use clap::Parser;
use fetch_core::store::lancedb::drop;

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
    // Name of table to drop
    #[arg(long)]
    table_name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    drop(args.data_directory.as_str(), args.table_name.as_str()).await?;

    println!("Completed dropping lancedb table at {}, {}", &args.data_directory, &args.table_name);

    Ok(())
}