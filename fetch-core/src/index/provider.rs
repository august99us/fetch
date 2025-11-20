use std::{future::Future, hash::{DefaultHasher, Hash, Hasher}};

use async_trait::async_trait;
use camino::{Utf8Path, Utf8PathBuf};
use chrono::{DateTime, Utc};
use log::debug;
use tokio::{fs, io};

use crate::{app_config::get_default_chunk_directory, index::ChunkFile};

#[async_trait]
pub trait ChunkingIndexProvider: Send + Sync {
    fn provides_indexing_for_extension(&self, ext: &str) -> bool;
    // I see no point to providing opt_modified on the index API, as we can always get it from 
    // the source of truth, the file itself.
    async fn index(&self, path: &Utf8Path, opt_modified: Option<DateTime<Utc>>) -> Result<(), IndexProviderError>;
    async fn clear(&self, path: &Utf8Path, opt_modified: Option<DateTime<Utc>>) -> Result<(), IndexProviderError>;
    async fn query_n(&self, str: &str, num_results: u32, offset: u32) -> Result<Vec<ChunkQueryResult>, IndexProviderError>;
}

pub struct ChunkQueryResult {
    chunkfile: ChunkFile,
    /// Normalized score value, ascending order. Higher = more relevant
    /// Implementers of ChunkingIndexProvider should target values between 0-100, but <100 is not guaranteed.
    /// There will be no negative values.
    score: f32,
}

impl ChunkQueryResult {
    pub fn new(chunkfile: ChunkFile, score: f32) -> Self {
        if score < 0.0 {
            panic!("Attempted creating a chunkfile with score < 0!");
        }

        ChunkQueryResult { chunkfile, score }
    }

    pub fn chunkfile(&self) -> &ChunkFile {
        &self.chunkfile
    }

    pub fn score(&self) -> f32 {
        self.score
    }
}

pub use error::*;

pub mod image;
pub mod error;

#[cfg(feature = "pdf")]
pub mod pdf;

// Private functions

/// Common function for generating the chunkfile dir from the original file, and making sure it exists
/// in the file system.
/// 
/// Will error if the tokio::fs::create_dir_all call errors
async fn create_chunkfile_dir(original_file_path: &Utf8Path) -> Result<Utf8PathBuf, io::Error> {
    // generate folder to store file chunks
    // TODO: create chunking module and refactor stuff into image-rs chunker,
    // pdfium chunker, etc.
    let chunk_out_dir = generate_chunkfile_dir_name(original_file_path);
    debug!("Creating chunkfile dir at {chunk_out_dir}");
    fs::create_dir_all(&chunk_out_dir).await?;

    Ok(chunk_out_dir)
}

async fn clear_chunkfiles(original_file_path: &Utf8Path) -> Result<(), io::Error> {
    let chunk_out_dir = generate_chunkfile_dir_name(original_file_path);

    debug!("Deleting directory with all chunkfiles at {chunk_out_dir}");
    fs::remove_dir_all(&chunk_out_dir).await
}

fn generate_chunkfile_dir_name(original_file_path: &Utf8Path) -> Utf8PathBuf {
    let chunk_data_dir = get_default_chunk_directory();
    let mut hasher = DefaultHasher::new();
    original_file_path.as_str().hash(&mut hasher);
    let filename_hash = hasher.finish().to_string();
    let chunk_out_dir = chunk_data_dir.join(filename_hash);

    chunk_out_dir
}