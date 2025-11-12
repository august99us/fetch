use std::{collections::HashMap, error::Error, sync::Arc};

use camino::Utf8PathBuf;
use clap::Parser;
use fetch_core::{app_config, files::{FileQueryer, pagination::QueryCursor, query::{QueryFiles, QueryResult}}, index::provider::{image::ImageIndexProvider, pdf::PdfIndexProvider}, init_ort, store::lancedb::LanceDBStore};

#[derive(Parser, Debug)]
#[command(name = "fetch-query")]
#[command(author = "August Sun, august99us@gmail.com")]
#[command(version = "0.0.2")]
#[command(about = "queries semantic file index with a query string", long_about = None)]
struct Args {
    /// Verbose mode
    #[arg(short, long)]
    verbose: bool,
    /// String to query files with
    query: String,
    /// The number of file results to return, default 20
    #[arg(short = 'n', long, default_value_t = 20)]
    num_results: u32,
    /// The number of chunks to query per API call (higher = faster but more memory), default 100
    #[arg(short = 'c', long, default_value_t = 100)]
    chunks_per_query: u32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    init_ort(None)?;
    env_logger::init();

    let data_dir = app_config::get_default_index_directory();

    // Create the image index store
    let siglip_store = Arc::new(LanceDBStore::local_full(
        data_dir.as_str(),
        "siglip2_chunkfile".to_owned()
    ).await
    .unwrap_or_else(|e| 
        panic!("Could not open lancedb store for image index with data dir: {}. Error: {e:?}",
        data_dir.as_str())));
    
    // Create the pdf index store
    let gemma_store = Arc::new(LanceDBStore::local_full(
        data_dir.as_str(),
        "gemma_chunkfile".to_owned()
    ).await
    .unwrap_or_else(|e| 
        panic!("Could not open lancedb store for pdf index with data dir: {}. Error: {e:?}",
        data_dir.as_str())));

    // Create the cursor store
    let cursor_store = LanceDBStore::<QueryCursor>::local(
        data_dir.as_str(),
        "cursor".to_owned()
    ).await
    .unwrap_or_else(|e|
        panic!("Could not open lancedb store for cursors with data dir: {}. Error: {e:?}",
        data_dir.as_str()));

    // Create index provider and file queryer
    let basic_image = ImageIndexProvider::using(siglip_store.clone());
    let pdf = PdfIndexProvider::using(gemma_store, siglip_store);
    let file_queryer = FileQueryer::with(vec![Arc::new(basic_image), Arc::new(pdf)], cursor_store);

    println!("Querying file index at {} with query: \"{}\"", data_dir.as_str(), args.query);

    // Aggregate results using cursor-based pagination
    let final_results = aggregate_results(&file_queryer, &args.query, args.num_results, args.chunks_per_query, args.verbose).await?;

    if final_results.is_empty() {
        println!("No results!");
    } else {
        println!("\nResults ({}):", final_results.len());
        for (i, result) in final_results.iter().enumerate() {
            println!("{}: {} (score: {:.2})", i + 1, result.path, result.score);
            }
    }

    Ok(())
}

/// Aggregates results by repeatedly calling the query API with cursor until we have enough results
/// or there are no more results available
async fn aggregate_results(
    queryer: &impl QueryFiles,
    query: &str,
    target_num_results: u32,
    chunks_per_query: u32,
    verbose: bool,
) -> Result<Vec<QueryResult>, Box<dyn Error>> {
    let mut cursor_id: Option<String> = None;
    let mut aggregated_results: HashMap<Utf8PathBuf, QueryResult> = HashMap::new();
    let mut iteration = 0;

    loop {
        iteration += 1;
        if verbose {
            println!("Query iteration {}, cursor: {:?}", iteration, cursor_id);
        }

        let result = queryer.query_n(query, chunks_per_query, cursor_id.as_deref()).await?;

        if verbose {
            println!("  Received {} changed results, total list length: {}",
                result.changed_results.len(), result.results_len);
        }

        // Update our aggregated results with the changed results
        for changed in result.changed_results {
            aggregated_results.insert(changed.path.clone(), changed);
        }

        // Check if we have enough results or if there's no more data
        if result.cursor_id.is_none() {
            if verbose {
                println!("No more results available (cursor exhausted)");
            }
            break;
        }

        if aggregated_results.len() >= target_num_results as usize {
            if verbose {
                println!("Target number of results ({}) reached", target_num_results);
            }
            break;
        }

        cursor_id = result.cursor_id;
    }

    // Convert to vec and sort by rank
    let mut final_results: Vec<QueryResult> = aggregated_results.into_values().collect();
    final_results.sort_by_key(|r| r.rank);

    // Truncate to target number of results
    final_results.truncate(target_num_results as usize);

    Ok(final_results)
}
