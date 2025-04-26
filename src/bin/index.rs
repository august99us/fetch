use std::{collections::HashSet, error::Error, path::{self, PathBuf}, sync::Arc, time::Duration};

use camino::Utf8PathBuf;
use clap::Parser;
use fetch::{file_index::{index_files::{FileIndexing, IndexFiles}, FileIndexer}, vector_store::{lancedb_store::LanceDBStore, IndexVector, QueryVectorKeys}};
use indicatif::ProgressBar;
use normalize_path::NormalizePath;
use tokio::{sync::Semaphore, task};

#[derive(Parser, Debug)]
#[command(name = "fetch-index")]
#[command(author = "August Sun, august99us@gmail.com")]
#[command(version = "0.1")]
#[command(about = "indexes things semantically", long_about = None)]
struct Args {
    // Verbose mode
    #[arg(short, long)]
    verbose: bool,
    /// Number of parallel indexing jobs to run at once
    #[arg(short, long, default_value_t = 10)]
    jobs: usize, // TODO
    /// Recursively look through sub folders to find files to index
    #[arg(short, long)]
    recursive: bool,
    /// Do not ask before indexing
    #[arg(short, long)]
    force: bool,
    /// File or folder paths to index
    paths: Vec<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    let classified_paths = classify_paths(args.paths);
    let mut files = classified_paths.files;

    explore_directories(classified_paths.folders, &mut files, args.recursive);

    let files = clean_paths(files);
    let unknown = clean_paths(classified_paths.unknown);

    if files.is_empty() {
        println!("No files to index! Goodbye.");
        return Ok(());
    }

    if !args.force {
        loop {
            println!("{} files discovered and queued for indexing - confirm? (Y/N)", files.len());
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
        println!("{} files discovered and queued for indexing.", files.len());
    }

    let lancedbstore = LanceDBStore::new("./data_dir", 512).await?;
    // TODO: unwrap error handling
    let file_indexer = Arc::new(FileIndexer::with(lancedbstore).unwrap());
    let files = files.into_iter().map(Arc::new).collect();

    let results = spawn_index_jobs(file_indexer, files, args.jobs).await;

    // run necessary processing for the "unknown" vector, the list of paths that are not files or directories or do not exist

    let mut success = 0;
    let mut fail = 0;
    for result in results {
        if let Ok(()) = result {
            success += 1;
        } else {
            fail += 1;
        }
    }

    println!("{} files successfully indexed, {} files failed.", success, fail);
    if fail > 0 {
        return Err(anyhow::Error::msg("oh no"));
    }

    Ok(())
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
    paths.into_iter().map(|pb| Utf8PathBuf::from_path_buf(pb)) // Convert to Utf8PathBuf
        .collect::<Result<Vec<Utf8PathBuf>, PathBuf>>() // collect results
        .unwrap_or_else(|e| panic!("Error verifying utf8 validity of path: {:?}", e))
}

/// Explores (io call) the paths given in "paths" vector and classifies them into one of three categories:
/// 1) files = path.is_file() is true
/// 2) folders = path.is_dir() is true
/// 3) unknown = neither is true
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
    while !queue.is_empty() {
        let folder = queue.pop().unwrap(); // guaranteed to exist
        if hashset.contains(&folder) {
            eprintln!("Warning: Circled back to folder that was already seen before. Maybe there is a symlink creating a circular 
                directory structure somewhere? Folder: {}", folder.to_str().expect("error converting pathbuf to string"));
                continue
        }
        for entry_result in folder.read_dir()
            .expect(&format!("failed reading directory: {}", folder.to_str().expect("error converting pathbuf to string"))) {
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

async fn spawn_index_jobs(file_indexer: Arc<FileIndexer<impl IndexVector + QueryVectorKeys + Sync + Send + 'static>>, 
    files: Vec<Arc<Utf8PathBuf>>, jobs: usize) -> Vec<Result<(), ()>> {
    let semaphore = Arc::new(Semaphore::new(jobs));
    let mut handles = vec![];

    let bar = Arc::new(ProgressBar::new(files.len().try_into().unwrap()));
    bar.enable_steady_tick(Duration::from_secs(1));
    bar.tick();

    for file in files {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let indexer_clone = file_indexer.clone();
        let bar_clone = bar.clone();
        let handle = task::spawn(async move {
            let result = indexer_clone.index(&file).await;

            drop(permit); // Release the permit when done
            bar_clone.inc(1);
            match result {
                Ok(FileIndexing::Result { path, r#type: FileIndexing::ResultType::Indexed }) => {
                    bar_clone.println(format!("File {} successfully indexed", path));
                    return Ok(());
                },
                Ok(FileIndexing::Result { path, r#type: FileIndexing::ResultType::Cleared  }) => {
                    bar_clone.println(format!("File {} not found or could not be previewed, successfully cleared from index", path));
                    return Ok(());
                },
                Err(e) => {
                    bar_clone.println(format!("Error while processing file with path {:?}: {:?}", e.path, e.source()));
                    return Err(());
                },
            }
        });
        handles.push(handle);
    }

    let mut results = vec![];
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    bar.finish();

    results
}