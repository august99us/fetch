use std::time::Duration;

use camino::{Utf8Path, Utf8PathBuf};
use crossbeam_channel::{unbounded, Receiver};
use fetch::{app_config, file_index::{index_files::IndexFiles, query_files::QueryFiles, FileIndexer}, vector_store::{lancedb_store::LanceDBStore, IndexVector}};
use notify::{event::{CreateKind, DataChange, ModifyKind}, EventKind, RecursiveMode};
use notify_debouncer_full::DebouncedEvent;
use tokio::fs;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), ()>{
    let worker_count = 4;

    // Create a channel to receive file change events
    let (tx, rx) = unbounded();

    // Create a watcher object
    let watcher_debouncer = notify_debouncer_full::new_debouncer(Duration::from_secs(2), None, tx);
    if watcher_debouncer.is_err() {
        eprintln!("Failed to create watcher: {:?}", watcher_debouncer.err());
        return Err(());
    }
    let mut watcher_debouncer = watcher_debouncer.unwrap();

    // Read paths from configuration file
    let watchlist_file = app_config::get_watchlist_file_path();
    println!("Reading watchlist file from: {watchlist_file}");

    // Read the watchlist file
    let watchlist = fs::read_to_string(watchlist_file).await;
    if watchlist.is_err() {
        eprintln!("Failed to read watchlist file: {:?}", watchlist.err());
        return Err(());
    }
    let watchlist = watchlist.unwrap();

    // Split the watchlist into individual paths
    let paths_to_watch: Vec<Utf8PathBuf> = watchlist.lines().map(Utf8PathBuf::from).collect();

    // Add paths to be watched (recursive mode)
    for path in paths_to_watch {
        let path = path.canonicalize_utf8()
            .unwrap_or_else(|e| panic!("Failed to canonicalize path: {path}, error: {e}"));
        watcher_debouncer.watch(path.as_std_path(), RecursiveMode::Recursive)
            .unwrap_or_else(|e| eprintln!("Failed to watch path: {path}, error: {e}"));
    }

    println!("File change tracking daemon is initiating workers...");

    let data_directory = app_config::get_default_data_directory();
    let vector_store = LanceDBStore::new(data_directory.as_str(), 512).await
        .unwrap_or_else(|e| panic!("Could not open lancedb store with data dir: ./data_dir. Error: {e:?}"));
    let file_indexer = FileIndexer::with(vector_store)
        .unwrap_or_else(|e| panic!("Failed to create file indexer: {e:?}"));

    let mut handles = Vec::with_capacity(worker_count);
    let cancellation_token = CancellationToken::new();

    for i in (0..worker_count) {
        println!("starting worker {i}...");
        let rx_clone = rx.clone();
        let token_clone = cancellation_token.clone();
        let file_indexer_clone = file_indexer.clone();
        let handle = tokio::spawn(worker_main(rx_clone, file_indexer_clone, token_clone));

        handles.push(handle);
    }

    match tokio::signal::ctrl_c().await {
        Ok(_) => println!("Received Ctrl+C, shutting down..."),
        Err(e) => eprintln!("Failed to listen for Ctrl+C: {e:?}"),
    }

    Ok(())
}

async fn worker_main<I: IndexFiles + QueryFiles>(rx: Receiver<Result<Vec<DebouncedEvent>, Vec<notify::Error>>>, 
    file_indexer: I, cancellation_token: CancellationToken) {
    while let Ok(event_message) = rx.recv() {
        if event_message.is_err() {
            eprintln!("Worker received error: {:?}", event_message.err());
            continue;
        }
        let events = event_message.unwrap();

        for event in events {
            handle_event(&file_indexer, event).await;
        }
    }
}

async fn handle_event<I: IndexFiles + QueryFiles>(file_indexer: &I, debounced_event: DebouncedEvent) {
    match debounced_event.event.kind {
        EventKind::Create(CreateKind::File) => {
            let file_path = <&Utf8Path>::try_from(debounced_event.event.paths.first()
                .expect("Expected at least one path for create file event")
                .as_path())
                .expect("Expected path to be valid UTF-8");
            println!("File created: {file_path}");

            // index file
            let result = file_indexer.index(file_path).await;
            match result {
                Ok(_) => println!("File indexed successfully: {file_path}"),
                Err(e) => eprintln!("Error indexing file {file_path}: {e:?}"),
            }
        },
        EventKind::Modify(ModifyKind::Data(DataChange::Any)) => {
            let file_path = <&Utf8Path>::try_from(debounced_event.event.paths.first()
                .expect("Expected at least one path for modify data event")
                .as_path())
                .expect("Expected path to be valid UTF-8");
            println!("File modified: {file_path:?}");

            // re-index file
            let result = file_indexer.index(file_path).await;
            match result {
                Ok(_) => println!("File updated successfully: {file_path}"),
                Err(e) => eprintln!("Error indexing file {file_path}: {e:?}"),
            }
        },
        EventKind::Modify(ModifyKind::Name(rename_mode)) => {
            println!("File renamed: {:?} with mode: {:?}", debounced_event.event.paths, rename_mode);
            let first_file_path = <&Utf8Path>::try_from(debounced_event.event.paths.first()
                .expect("Expected at least one path for modify name event")
                .as_path())
                .expect("Expected path to be valid UTF-8");
            let second_file_path = debounced_event.event.paths.get(1)
                .map(|p| <&Utf8Path>::try_from(p.as_path()).expect("Expected path to be valid UTF-8"));
            if second_file_path.is_some() {
                println!("Two paths found. File renamed: {:?} to {:?}", first_file_path, second_file_path.unwrap());
                let clear_future = file_indexer.clear(first_file_path);
                let index_future = file_indexer.index(second_file_path.unwrap());
                match clear_future.await {
                    Ok(_) => println!("File cleared from index: {first_file_path}"),
                    Err(e) => eprintln!("Error clearing file {first_file_path}: {e:?}"),
                }
                match index_future.await {
                    Ok(_) => println!("File indexed successfully: {:?}", second_file_path.unwrap()),
                    Err(e) => eprintln!("Error indexing file {}: {:?}", second_file_path.unwrap(), e),
                }
            } else {
                println!("File renamed: {first_file_path:?}. Unknown whether this is the 'to' or 'from' name.");
                let result = file_indexer.index(first_file_path).await;
                match result {
                    Ok(_) => println!("File updated successfully (could be delete): {first_file_path}"),
                    Err(e) => eprintln!("Error indexing file {first_file_path}: {e:?}"),
                }
            }
        },
        EventKind::Remove(_) => {
            let file_path = <&Utf8Path>::try_from(debounced_event.event.paths.first()
                .expect("Expected at least one path for delete file event")
                .as_path())
                .expect("Expected path to be valid UTF-8");
            println!("File removed: {file_path:?}");

            let result = file_indexer.clear(file_path).await;
            match result {
                Ok(_) => println!("File cleared from index: {file_path}"),
                Err(e) => eprintln!("Error clearing file {file_path}: {e:?}"),
            }
        },
        EventKind::Access(_) => {
            println!("File(s) accessed: {:?}, ignoring", debounced_event.event.paths);
        },
        _ => {
            eprintln!("Unhandled event kind: {:?}", debounced_event.event.kind);
        },
    }
}