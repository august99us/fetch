use std::{error::Error, future::Future, pin::Pin};

use clap::Parser;
use fetch_core::{app_config, embeddable::session_pool::init_querying, file_index::{query_files::{FileQuerying, QueryFiles}, FileIndexer}, vector_store::lancedb_store::LanceDBStore};
use fetch_util::bin::init_ort;

#[derive(Parser, Debug)]
#[command(name = "fetch-query")]
#[command(author = "August Sun, august99us@gmail.com")]
#[command(version = "0.1")]
#[command(about = "queries semantic file index with a query string", long_about = None)]
struct Args {
    /// Verbose mode
    #[arg(short, long)]
    verbose: bool,
    /// String to query files with
    query: String,
    /// Page number for results (1-based), default 1
    #[arg(short, long)]
    page: Option<u32>,
    /// The number of query results to return per page, default 20
    #[arg(short, long)]
    num_results: Option<u32>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    init_ort()?;
    init_querying();

    let data_dir = app_config::get_default_index_directory();
    let lancedbstore = LanceDBStore::new(data_dir.as_str(), 768).await
        .unwrap_or_else(|e| panic!("Could not open lancedb store with data dir: {}. Error: {e:?}", data_dir.as_str()));
    let file_indexer = FileIndexer::with(lancedbstore);

    println!("Querying file index at {} with query: \"{}\"", data_dir.as_str(), args.query);

    let result_future: Pin<Box<dyn Future<Output = Result<FileQuerying::Result, FileQuerying::Error>>>>;
    if let Some(n) = args.num_results {
        result_future = Box::pin(file_indexer.query_n(&args.query, n, args.page.unwrap_or(1)));
    } else {
        result_future = Box::pin(file_indexer.query(&args.query, args.page));
    }

    let results = result_future.await
        .unwrap_or_else(|e| panic!("Issue querying file index: {e:?}"));

    if results.is_empty() {
        println!("No results!");
    } else {
        println!("Results ({}):", results.len());
        for (i, result) in results.iter().enumerate() {
            println!("{}: {}, {}", i + 1, result.path, result.similarity);
        }
    }

    Ok(())
}
