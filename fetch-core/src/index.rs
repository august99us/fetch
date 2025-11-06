use async_trait::async_trait;
use camino::{Utf8Path, Utf8PathBuf};
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

use crate::store::KeyedSequencedData;

pub struct ChunkFile {
    // Composite key
    pub original_file: Utf8PathBuf,
    pub chunk_channel: String,
    pub chunk_sequence_id: f32,
    // Other data pieces
    pub chunkfile: Utf8PathBuf,
    pub chunk_type: ChunkType,
    pub chunk_length: f32,
    pub original_file_creation_date: DateTime<Utc>,
    pub original_file_modified_date: DateTime<Utc>,
    pub original_file_size: u64,
    pub original_file_tags: Map<String, Value>,
}

pub enum ChunkType {
    Text,
    Image,
    Video,
    Audio,
}

#[async_trait]
pub trait ChunkingIndexProvider: Send + Sync {
    fn provides_indexing_for_extension(&self, ext: &str) -> bool;
    async fn index(&self, path: &Utf8Path) -> Result<(), anyhow::Error>;
    async fn clear(&self, path: &Utf8Path) -> Result<(), anyhow::Error>;
    async fn query_n(&self, str: &str, num_results: u32, offset: u32) -> Result<Vec<ChunkQueryResult>, anyhow::Error>;
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

impl KeyedSequencedData<String> for ChunkFile {
    fn get_key(&self) -> String {
        // Create a unique key from the composite key fields
        format!("{}::{}::{}",
            self.original_file,
            self.chunk_channel,
            self.chunk_sequence_id)
    }

    fn get_sequence_num(&self) -> u64 {
        // Use modification timestamp as sequence number for versioning
        // Higher values = newer versions
        // Note: timestamp_millis() returns i64, which overflows around year 292,277,026 CE
        // (approximately 292 million years from Unix epoch). This cast to u64 is safe for
        // all reasonable file modification dates after 1970-01-01.
        self.original_file_modified_date.timestamp_millis() as u64
    }
}

pub mod basic_image_index_provider;
pub mod embedding;

pub use integrations::*;

// Private variables and functions
mod integrations;