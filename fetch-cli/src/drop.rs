use std::{error::Error, path::PathBuf};

use camino::Utf8PathBuf;
use fetch_core::store::lancedb;

pub struct DropArgs {
    // Directory where index is stored
    pub data_directory: PathBuf,
    // Name of table to drop
    pub table_name: String,
}

pub async fn drop(args: DropArgs) -> Result<(), Box<dyn Error>> {
    let data_dir = Utf8PathBuf::from_path_buf(args.data_directory)
        .expect("data_directory path is not valid UTF-8");

    lancedb::drop(data_dir.as_str(), args.table_name.as_str()).await?;

    println!("Completed dropping lancedb table at {}, {}", &data_dir, &args.table_name);

    Ok(())
}