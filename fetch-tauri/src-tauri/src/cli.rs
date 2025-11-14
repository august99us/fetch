use std::{collections::HashMap, error::Error, path::PathBuf};

use fetch_cli::{drop::DropArgs, index::IndexArgs, query::QueryArgs, query_by_file::QueryByFileArgs};
use tauri::AppHandle;
use tauri_plugin_cli::{ArgData, CliExt};

/// Checks to see if we are running a CLI program, then executes it if so. Returns
/// true if CLI command was detected.
pub fn intercept_cli_command(app_handle: &AppHandle) -> bool {
    println!("Intercepting CLI command...");
    if let Ok(matches) = app_handle.cli().matches() {
        check_help_and_maybe_exit(app_handle, &matches.args);
        if let Some(subcommand) = matches.subcommand {
            let rt = tokio::runtime::Runtime::new().expect("Unable to create runtime");
            let result: Result<(), Box<dyn Error>> = rt.block_on(async move {
                let sc_args = subcommand.matches.args;
                check_help_and_maybe_exit(app_handle, &sc_args);
                match subcommand.name.as_str() {
                    "drop" => {
                        let args = DropArgs {
                            data_directory: PathBuf::from(sc_args
                                .get("data_directory")
                                .expect("subcommand was 'drop' but data_directory arg does not exist")
                                .value
                                .as_str()
                                .expect("Could not get data_directory arg as string")),
                            table_name: sc_args
                                .get("table_name")
                                .expect("subcommand was 'drop' but table_name arg does not exist")
                                .value
                                .as_str()
                                .expect("Could not get table_name arg as string")
                                .to_owned(),
                        };

                        fetch_cli::drop::drop(args).await?;
                    },
                    "index" => {
                        let jobs: usize = sc_args
                            .get("jobs")
                            .and_then(|arg| arg.value.as_str())
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(4);

                        let recursive = sc_args.contains_key("recursive");
                        let force = sc_args.contains_key("force");

                        let paths: Vec<PathBuf> = sc_args
                            .get("paths")
                            .and_then(|arg| arg.value.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(PathBuf::from)).collect())
                            .unwrap_or_default();

                        let args = IndexArgs {
                            jobs,
                            recursive,
                            force,
                            paths,
                        };

                        fetch_cli::index::index(args).await?;
                    },
                    "query" => {
                        let query = sc_args
                            .get("query")
                            .expect("subcommand was 'query' but query arg does not exist")
                            .value
                            .as_str()
                            .expect("Could not get query arg as string")
                            .to_owned();

                        let num_results: u32 = sc_args
                            .get("num_results")
                            .and_then(|arg| arg.value.as_str())
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(20);

                        let chunks_per_query: u32 = sc_args
                            .get("chunks_per_query")
                            .and_then(|arg| arg.value.as_str())
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(100);

                        let args = QueryArgs {
                            query,
                            num_results,
                            chunks_per_query,
                        };

                        fetch_cli::query::query(args).await?;
                    },
                    "query-by-file" => {
                        let query = PathBuf::from(sc_args
                            .get("query")
                            .expect("subcommand was 'query-by-file' but query arg does not exist")
                            .value
                            .as_str()
                            .expect("Could not get query arg as string"));

                        let num_results: u32 = sc_args
                            .get("num_results")
                            .and_then(|arg| arg.value.as_str())
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(20);

                        let args = QueryByFileArgs {
                            query,
                            num_results,
                        };

                        fetch_cli::query_by_file::query_by_file(args).await?;
                    },
                    _ => panic!("Invalid cli subcommand name"),
                }
                
                Ok(())
            });

            match result {
                Ok(_) => app_handle.exit(0),
                Err(e) => {
                    eprintln!("{:?}", e);
                    app_handle.exit(1);
                },
            }

            return true;
        }
    }

    return false;
}

fn check_help_and_maybe_exit(app_handle: &AppHandle, args: &HashMap<String, ArgData>) {
    if let Some(message) = args.get("help") {
        println!("{}", message.value.as_str().unwrap());
        app_handle.exit(0);
    }
}