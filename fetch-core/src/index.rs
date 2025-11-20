use camino::Utf8PathBuf;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

use crate::store::KeyedSequencedData;

// TODO: update sequence number to separate value from file modified date - chunkfile creation date?
// Will require complete regeneration of database
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

#[derive(Debug, PartialEq)]
pub enum ChunkType {
    Text,
    Image,
    Video,
    Audio,
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

pub mod provider;
pub mod embedding;

pub use integrations::*;

// Private variables and functions
mod integrations;