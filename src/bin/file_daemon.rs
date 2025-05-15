use std::time::Duration;

use camino::Utf8PathBuf;
use fetch::app_config;
use notify::{event::{CreateKind, DataChange, ModifyKind}, Event, EventKind, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, DebouncedEvent};
use tokio::{fs, sync::mpsc::unbounded_channel};

#[tokio::main]
async fn main() -> Result<(), ()>{
    // Create a channel to receive file change events
    let (tx, rx) = unbounded_channel();

    // Create a watcher object
    let mut watcher_debouncer = notify_debouncer_full::new_debouncer(Duration::from_secs(2), None,
        move |result: DebounceEventResult| {
            match result {
                Ok(events) => {
                    let result = tx.send(events.into_iter().last().expect("Empty event list passed to debounce handler"));
                    if result.is_err() {
                        eprintln!("Failed to send event to channel: {:?}", result.err());
                    }
                },
                Err(e) => eprintln!("Watcher/debouncer error(s): {:?}", e),
            }
        });
    if watcher_debouncer.is_err() {
        eprintln!("Failed to create watcher: {:?}", watcher_debouncer.err());
        return Err(());
    }
    let watcher_debouncer = watcher_debouncer.unwrap();

    // Read paths from configuration file
    let config = app_config::get_daemon_config();
    if config.is_err() {
        eprintln!("Failed to load daemon config file: {:?}", config.err());
        return Err(());
    }
    let config = config.unwrap();

    // Get the file path of the daemon file watchlist from the config
    let watchlist_file = config.get_string("daemon_watchlist_file").map(Utf8PathBuf::from);
    if watchlist_file.is_err() {
        eprintln!("Failed to get watchlist file from config: {:?}", watchlist_file.err());
        return Err(());
    }
    let watchlist_file = watchlist_file.unwrap();

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
        watcher_debouncer.watch(path.as_std_path(), RecursiveMode::Recursive)
            .unwrap_or_else(|e| eprintln!("Failed to watch path: {}, error: {}", path, e));
    }

    println!("File change tracking daemon is running...");

    // Event loop to process file changes
    loop {
        match rx.recv().await {
            Some(event) => match event {
                DebouncedEvent { event: Event {
                        paths,
                        kind: EventKind::Create(CreateKind::File),
                        attrs,
                    }, time } => {
                        println!(
                            "File created: {} at time: {}",
                            paths.last(),
                            DateTime::Local::from(time).format("%Y-%m-%d %H:%M:%S")
                        );
                    }
                },
                DebouncedEvent { event: Event {
                        paths,
                        kind: EventKind::Modify(ModifyKind::Data(DataChange::Any)),
                        attrs,
                    }, time } => {
                        println!("File modified: {} at time: {}", paths.last(), time);
                },
                _ => println!("something"),
            },
            None => {
                println!("Channel closed");
                break;
            },
        }
    }

    Ok(())
}