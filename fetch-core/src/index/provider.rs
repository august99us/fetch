use async_trait::async_trait;
use camino::Utf8Path;
use chrono::{DateTime, Utc};

use crate::index::ChunkFile;

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