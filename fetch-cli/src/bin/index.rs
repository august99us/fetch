use std::{collections::HashSet, error::Error, path::{self, PathBuf}, sync::Arc, time::Duration};

use camino::Utf8PathBuf;
use clap::Parser;
use fetch_cli::utility::print_metrics;
use fetch_core::{app_config, init_ort, file_index::{index_files::{IndexFiles, FileIndexingResult, FileIndexingResultType}, FileIndexer}, index::basic_image_index_provider::BasicImageIndexProvider, store::lancedb::LanceDBStore};
use indicatif::ProgressBar;
use normalize_path::NormalizePath;
use tokio::{runtime, sync::Semaphore, task};

#[derive(Parser, Debug)]
#[command(name = "fetch-index")]
#[command(author = "August Sun, august99us@gmail.com")]
#[command(version = "0.1")]
#[command(about = "indexes things semantically", long_about = None)]
struct Args {
    /// Verbose mode
    #[arg(short, long)]
    verbose: bool,
    /// Number of parallel indexing jobs to run at once
    #[arg(short, long, default_value_t = 4)]
    jobs: usize,
    /// Recursively look through sub folders to find files to index
    #[arg(short, long)]
    recursive: bool,
    /// Do not confirm before indexing
    #[arg(short, long)]
    force: bool,
    /// Track and print metrics
    #[arg(short, long)]
    metrics: bool,
    /// File or folder paths to index
    paths: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    init_ort(None)?;
    env_logger::init();

    let args = Args::parse();
    let start_time = std::time::Instant::now();

    let rt = runtime::Builder::new_multi_thread()
        // Worker threads do not necessarily determine how many blocking tasks can be spawned.
        // Blocking tasks in tokio include stuff like io tasks. Limiting the number of blocking tasks
        // will also limit those io tasks in the same pool as the actual cpu blocking tasks Previewable
        // and Embeddable need, which run for much longer. It would be better to run those in rayon
        // or just run a maximum of 4 indexing jobs at once, using a semaphore.
        // .worker_threads(args.jobs)
        .enable_all()
        .build()
        .expect("Failed to create runtime");

    let result = rt.block_on(async move {
        let classified_paths = classify_paths(args.paths);
        let mut files = classified_paths.files;

        explore_directories(classified_paths.folders, &mut files, args.recursive);

        let files = clean_paths(files);
        // files classified as unknown are likely paths that were deleted and need to be cleared
        let unknown = clean_paths(classified_paths.unknown);

        if files.is_empty() && unknown.is_empty() {
            println!("Nothing to do! Goodbye.");
            return Ok(());
        }

        if !args.force {
            loop {
                println!("{} file(s) discovered.\n\
                    {} queued for indexing.\n\
                    {} queued for clearing.\n\
                    Confirm? (Y/N)",
                    files.len() + unknown.len(),
                    files.len(),
                    unknown.len());
                let mut confirmation = String::new();
                std::io::stdin().read_line(&mut confirmation).expect("Failed to read line");
                
                // Trim the confirmation to remove any extra whitespace or newline characters
                let confirmation = confirmation.trim();
                match confirmation {
                    "Y" | "y" | "yes" | "Yes" => break,
                    "N" | "n" | "no" | "No" => {
                        println!("Aborting...");
                        return Ok(());
                    },
                    _ => println!("Unrecognized input entered. Please try again."),
                }
            } 
            println!("Proceeding with indexing {} files.", files.len())
        } else {
            println!("{} files discovered.\n\
                {} queued for indexing.\n\
                {} queued for clearing.",
                files.len() + unknown.len(),
                files.len(),
                unknown.len());
        }

        let data_dir = app_config::get_default_index_directory();
        let siglip_store = LanceDBStore::local_full(
            data_dir.as_str(),
            "siglip2_chunkfile".to_owned()
        ).await
        .unwrap_or_else(|e| panic!("Could not open lancedb store with data dir: {}. Error: {e:?}", data_dir.as_str()));
        let basic_image = BasicImageIndexProvider::using(siglip_store);
        let file_indexer: Arc<FileIndexer> = Arc::new(FileIndexer::with(vec![Arc::new(basic_image)]));

        println!("Indexing {} files into index stored in the directory {} with {} parallel jobs",
            files.len(),
            data_dir.as_str(),
            args.jobs);
        let iresults = spawn_index_jobs(file_indexer.clone(), files, args.jobs).await;
        let mut isuccess = 0;
        let mut ifail = 0;
        for result in iresults {
            if let Ok(()) = result {
                isuccess += 1;
            } else {
                ifail += 1;
            }
        }

        println!("Clearing {} unknown files from index stored in directory {} with {} parallel jobs",
            unknown.len(),
            data_dir.as_str(),
            args.jobs);
        let cresults = spawn_clear_jobs(file_indexer, unknown, args.jobs).await;
        let mut csuccess = 0;
        let mut cfail = 0;
        for result in cresults {
            if let Ok(()) = result {
                csuccess += 1;
            } else {
                cfail += 1;
            }
        }

        println!("{isuccess} files successfully indexed, {ifail} files failed indexing.");
        println!("{csuccess} files successfully cleared, {cfail} files failed clearing.");
        if ifail > 0 || cfail > 0 {
            return Err(anyhow::Error::msg("oh no"));
        }

        Ok(())
    });

    if args.metrics {
        print_metrics(&rt.metrics());

        let elapsed = start_time.elapsed();
        println!("Total indexing duration: {:.2?}", elapsed);
        println!("Press Enter to quit...");
        let mut empty = String::new();
        let _ = std::io::stdin().read_line(&mut empty);
    }

    Ok(result?)
}

/// Sanitizes, sorts, and dedupes a vec of PathBufs into Utf8PathBufs
fn clean_paths(paths: Vec<PathBuf>) -> Vec<Utf8PathBuf> {
    let mut paths = paths.into_iter() // consume vec and iter
        .map(|pb| path::absolute(pb) // convert path to absolute path if relative
            .map(|ap| ap.normalize())) // normalize the absolute path
        .collect::<Result<Vec<PathBuf>, std::io::Error>>() // collect
        .expect("Could not get current directory to convert path to absolute path"); // propagate error
        // Technically, the path::absolute() function can error on two things: 1) can't get current error, or
        // 2) path is empty. (https://doc.rust-lang.org/stable/std/path/fn.absolute.html) We don't have to worry about
        // the "path is empty" situation because clap will not fill the args with a value if the provided argument
        // is empty.
    paths.sort();
    paths.dedup();
    paths.into_iter().map(Utf8PathBuf::from_path_buf) // Convert to Utf8PathBuf
        .collect::<Result<Vec<Utf8PathBuf>, PathBuf>>() // collect results
        .unwrap_or_else(|e| panic!("Error verifying utf8 validity of path: {e:?}"))
}

/// Explores (io call) the paths given in "paths" vector and classifies them into one of three categories:
/// 1) files = path.is_file() is true
/// 2) folders = path.is_dir() is true
/// 3) unknown = neither is true
/// 
/// Returns classified paths in a struct
struct ClassifiedPaths {
    pub files: Vec<PathBuf>,
    pub folders: Vec<PathBuf>,
    pub unknown: Vec<PathBuf>,
}
fn classify_paths(paths: Vec<PathBuf>) -> ClassifiedPaths {
    let mut classified = ClassifiedPaths { files: vec![], folders: vec![], unknown: vec![] };
    paths.into_iter().for_each(|path| {
        if path.is_file() {
            classified.files.push(path);
        } else if path.is_dir() {
            classified.folders.push(path);
        } else {
            classified.unknown.push(path);
        }
    });
    classified
}

/// Expands the directories given in "folders", adding the files found to the "files" vec. Will recursively
/// explore directories found within those folders as well if recursive = true
fn explore_directories(folders: Vec<PathBuf>, files: &mut Vec<PathBuf>, recursive: bool) {
    let mut hashset: HashSet<PathBuf> = HashSet::new();
    let mut queue = folders;
    while let Some(folder) = queue.pop() {
         // guaranteed to exist
        if hashset.contains(&folder) {
            eprintln!("Warning: Circled back to folder that was already seen before. Maybe there is a symlink creating a circular 
                directory structure somewhere? Folder: {}", folder.to_str().expect("error converting pathbuf to string"));
                continue
        }
        for entry_result in folder.read_dir()
            .unwrap_or_else(|_| panic!("failed reading directory: {}", folder.to_str().expect("error converting pathbuf to string"))) {
            match entry_result {
                Ok(entry) => {
                    let entry_path = entry.path();
                    if entry_path.is_file() {
                        files.push(entry_path);
                    } else if entry_path.is_dir() {
                        if recursive {
                            queue.push(entry_path);
                        } else {
                            eprintln!("Warning: subdirectory found when reading directory but recursive flag missing, ignoring: {}",
                                entry_path.to_str().expect("error converting pathbuf to string"));
                        }
                    } else {
                        eprintln!("Warning: directory entry that is not a file nor a directory found: {}", entry_path.to_str()
                            .expect("error converting pathbuf to string"));
                    }
                },
                Err(e) => panic!("Issue reading directory entry: {e:?}"),
            }
        }
        hashset.insert(folder);
    }
}

async fn spawn_index_jobs(file_indexer: Arc<impl IndexFiles + Sync + Send + Clone + 'static>,
    files: Vec<Utf8PathBuf>, jobs: usize) -> Vec<Result<(), ()>> {
    let semaphore = Arc::new(Semaphore::new(jobs));
    let mut handles = vec![];

    let bar = Arc::new(ProgressBar::new(files.len().try_into().unwrap()));
    bar.enable_steady_tick(Duration::from_secs(1));
    bar.tick();

    for file in files {
        let permit = semaphore.clone().acquire_owned().await.unwrap_or_else(|e| 
            panic!("Failed to acquire semaphore permit (was the semaphore closed?): {e:?}"));
        let indexer_clone = file_indexer.clone();
        let bar_clone = bar.clone();
        let handle = task::spawn(async move {
            let result = indexer_clone.index(&file).await;

            drop(permit); // Release the permit when done
            bar_clone.inc(1);
            match result {
                Ok(FileIndexingResult { path, r#type: FileIndexingResultType::Indexed }) => {
                    bar_clone.println(format!("File {path} successfully indexed"));
                    Ok(())
                },
                Ok(FileIndexingResult { path, r#type: FileIndexingResultType::Cleared  }) => {
                    bar_clone.println(format!("File {path} not found or could not be previewed, successfully cleared from index"));
                    Ok(())
                },
                Err(e) => {
                    bar_clone.println(format!("Error while processing file with path {:?}: {:?}", e.path, e.source()));
                    Err(())
                },
            }
        });
        handles.push(handle);
    }

    let mut results = vec![];
    for handle in handles {
        results.push(handle.await.unwrap_or(Err(())));
    }

    bar.finish();

    results
}

async fn spawn_clear_jobs(file_indexer: Arc<impl IndexFiles + Sync + Send + Clone + 'static>,
    files: Vec<Utf8PathBuf>, jobs: usize) -> Vec<Result<(), ()>> {
    let semaphore = Arc::new(Semaphore::new(jobs));
    let mut handles = vec![];

    let bar = Arc::new(ProgressBar::new(files.len().try_into().unwrap()));
    bar.enable_steady_tick(Duration::from_secs(1));
    bar.tick();

    for file in files {
        let permit = semaphore.clone().acquire_owned().await.unwrap_or_else(|e| 
            panic!("Failed to acquire semaphore permit (was the semaphore closed?): {e:?}"));
        let indexer_clone = file_indexer.clone();
        let bar_clone = bar.clone();
        let handle = task::spawn(async move {
            let result = indexer_clone.clear(&file).await;

            drop(permit); // Release the permit when done
            bar_clone.inc(1);
            match result {
                Ok(FileIndexingResult { path: _, r#type: FileIndexingResultType::Indexed }) => {
                    unreachable!("Clear will never return an Indexed result");
                },
                Ok(FileIndexingResult { path, r#type: FileIndexingResultType::Cleared  }) => {
                    bar_clone.println(format!("Path {path} successfully cleared from index"));
                    Ok(())
                },
                Err(e) => {
                    bar_clone.println(format!("Error while clearing file with path {:?}: {:?}", e.path, e.source()));
                    Err(())
                },
            }
        });
        handles.push(handle);
    }

    let mut results = vec![];
    for handle in handles {
        results.push(handle.await.unwrap_or(Err(())));
    }

    bar.finish();

    results
}