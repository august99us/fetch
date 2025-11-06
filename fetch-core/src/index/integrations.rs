use std::sync::{Arc, LazyLock};
use arrow::array::{StringBuilder, Float32Builder, UInt64Builder, TimestampMillisecondBuilder, AsArray};
use arrow::datatypes::{Float32Type, TimestampMillisecondType, UInt64Type};
use arrow_array::{RecordBatch, ArrayRef};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use camino::Utf8PathBuf;
use chrono::{Utc, TimeZone};
use serde_json::Value;
use serde_json::Map;

use crate::index::{ChunkFile, ChunkType};
use crate::store::{FTSData, Filterable, lancedb::{ArrowData, RowBuilder}};

// Chunkfile ArrowData integrations

// Attribute names (field names on the ChunkFile struct)
pub const ORIGINAL_FILE_ATTR: &str = "original_file";
pub const CHUNK_CHANNEL_ATTR: &str = "chunk_channel";
pub const CHUNK_SEQUENCE_ID_ATTR: &str = "chunk_sequence_id";
pub const CHUNKFILE_ATTR: &str = "chunkfile";
pub const CHUNK_TYPE_ATTR: &str = "chunk_type";
pub const CHUNK_LENGTH_ATTR: &str = "chunk_length";
pub const FILE_CREATION_DATE_ATTR: &str = "original_file_creation_date";
pub const FILE_MODIFIED_DATE_ATTR: &str = "original_file_modified_date";
pub const FILE_SIZE_ATTR: &str = "original_file_size";
pub const FILE_TAGS_ATTR: &str = "original_file_tags";

// Column names (Arrow schema column names)
const ORIGINAL_FILE_COLUMN_NAME: &str = "original_file";
const CHUNK_CHANNEL_COLUMN_NAME: &str = "chunk_channel";
const CHUNK_SEQUENCE_ID_COLUMN_NAME: &str = "chunk_sequence_id";
const CHUNKFILE_COLUMN_NAME: &str = "chunkfile";
const CHUNK_TYPE_COLUMN_NAME: &str = "chunk_type";
const CHUNK_LENGTH_COLUMN_NAME: &str = "chunk_length";
const FILE_CREATION_DATE_COLUMN_NAME: &str = "original_file_creation_date";
const FILE_MODIFIED_DATE_COLUMN_NAME: &str = "original_file_modified_date";
const FILE_SIZE_COLUMN_NAME: &str = "original_file_size";
const FILE_TAGS_COLUMN_NAME: &str = "original_file_tags";

static ORIGINAL_FILE_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(ORIGINAL_FILE_COLUMN_NAME, DataType::Utf8, false))
});
static CHUNK_CHANNEL_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(CHUNK_CHANNEL_COLUMN_NAME, DataType::Utf8, false))
});
static CHUNK_SEQUENCE_ID_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(CHUNK_SEQUENCE_ID_COLUMN_NAME, DataType::Float32, false))
});
static CHUNKFILE_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(CHUNKFILE_COLUMN_NAME, DataType::Utf8, false))
});
static CHUNK_TYPE_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(CHUNK_TYPE_COLUMN_NAME, DataType::Utf8, false))
});
static CHUNK_LENGTH_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(CHUNK_LENGTH_COLUMN_NAME, DataType::Float32, false))
});
static FILE_CREATION_DATE_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(FILE_CREATION_DATE_COLUMN_NAME, DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into())), false))
});
static FILE_MODIFIED_DATE_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(FILE_MODIFIED_DATE_COLUMN_NAME, DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into())), false))
});
static FILE_SIZE_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(FILE_SIZE_COLUMN_NAME, DataType::UInt64, false))
});
static FILE_TAGS_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(FILE_TAGS_COLUMN_NAME, DataType::Utf8, false))
});

static CHUNKFILE_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    Schema::new(vec![
        ORIGINAL_FILE_FIELD.clone(),
        CHUNK_CHANNEL_FIELD.clone(),
        CHUNK_SEQUENCE_ID_FIELD.clone(),
        CHUNKFILE_FIELD.clone(),
        CHUNK_TYPE_FIELD.clone(),
        CHUNK_LENGTH_FIELD.clone(),
        FILE_CREATION_DATE_FIELD.clone(),
        FILE_MODIFIED_DATE_FIELD.clone(),
        FILE_SIZE_FIELD.clone(),
        FILE_TAGS_FIELD.clone(),
    ])
});

pub struct ChunkFileRowBuilder {
    original_file: StringBuilder,
    chunk_channel: StringBuilder,
    chunk_sequence_id: Float32Builder,
    chunkfile: StringBuilder,
    chunk_type: StringBuilder,
    chunk_length: Float32Builder,
    original_file_creation_date: TimestampMillisecondBuilder,
    original_file_modified_date: TimestampMillisecondBuilder,
    original_file_size: UInt64Builder,
    original_file_tags: StringBuilder,
}

impl ChunkFileRowBuilder {
    pub fn new() -> Self {
        Self {
            original_file: StringBuilder::new(),
            chunk_channel: StringBuilder::new(),
            chunk_sequence_id: Float32Builder::new(),
            chunkfile: StringBuilder::new(),
            chunk_type: StringBuilder::new(),
            chunk_length: Float32Builder::new(),
            original_file_creation_date: TimestampMillisecondBuilder::new().with_timezone("UTC"),
            original_file_modified_date: TimestampMillisecondBuilder::new().with_timezone("UTC"),
            original_file_size: UInt64Builder::new(),
            original_file_tags: StringBuilder::new(),
        }
    }
}

impl RowBuilder<ChunkFile> for ChunkFileRowBuilder {
    fn append(&mut self, row: ChunkFile) {
        self.original_file.append_value(row.original_file.as_str());
        self.chunk_channel.append_value(&row.chunk_channel);
        self.chunk_sequence_id.append_value(row.chunk_sequence_id);
        self.chunkfile.append_value(row.chunkfile.as_str());
        self.chunk_type.append_value(chunk_type_to_string(row.chunk_type));
        self.chunk_length.append_value(row.chunk_length);
        self.original_file_creation_date.append_value(row.original_file_creation_date.timestamp_millis());
        self.original_file_modified_date.append_value(row.original_file_modified_date.timestamp_millis());
        self.original_file_size.append_value(row.original_file_size);

        // Serialize tags as JSON string
        let tags_json = serde_json::to_string(&row.original_file_tags).unwrap_or_else(|_| "{}".to_string());
        self.original_file_tags.append_value(&tags_json);
    }

    fn finish(mut self) -> Vec<(Arc<Field>, ArrayRef)> {
        vec![
            (ORIGINAL_FILE_FIELD.clone(), Arc::new(self.original_file.finish())),
            (CHUNK_CHANNEL_FIELD.clone(), Arc::new(self.chunk_channel.finish())),
            (CHUNK_SEQUENCE_ID_FIELD.clone(), Arc::new(self.chunk_sequence_id.finish())),
            (CHUNKFILE_FIELD.clone(), Arc::new(self.chunkfile.finish())),
            (CHUNK_TYPE_FIELD.clone(), Arc::new(self.chunk_type.finish())),
            (CHUNK_LENGTH_FIELD.clone(), Arc::new(self.chunk_length.finish())),
            (FILE_CREATION_DATE_FIELD.clone(), Arc::new(self.original_file_creation_date.finish())),
            (FILE_MODIFIED_DATE_FIELD.clone(), Arc::new(self.original_file_modified_date.finish())),
            (FILE_SIZE_FIELD.clone(), Arc::new(self.original_file_size.finish())),
            (FILE_TAGS_FIELD.clone(), Arc::new(self.original_file_tags.finish())),
        ]
    }
}

impl ArrowData for ChunkFile {
    type RowBuilder = ChunkFileRowBuilder;

    fn schema() -> Schema {
        CHUNKFILE_SCHEMA.clone()
    }

    fn row_builder() -> Self::RowBuilder {
        ChunkFileRowBuilder::new()
    }

    fn batch_to_iter(record_batch: RecordBatch) -> impl IntoIterator<Item = Self> {
        let num_rows = record_batch.num_rows();

        // Move record_batch into the iterator by capturing it in the closure
        // This is necessary because the serde_json::from_str function itself captures the
        // lifetime of the &str it is deserializing from, so the original owner of the data
        // (record batch -> &ByteArray -> &json str) needs to life for as long as the iterator
        (0..num_rows).map(move |i| {
            // ::<i32> specifies 32-bit offsets for string arrays (matching DataType::Utf8 in schema).
            // Use ::<i64> only for LargeUtf8 arrays with >2GB total string data.
            let original_file = record_batch.column_by_name(ORIGINAL_FILE_COLUMN_NAME)
                .expect("original_file column not found")
                .as_string::<i32>()
                .value(i);
            let chunk_channel = record_batch.column_by_name(CHUNK_CHANNEL_COLUMN_NAME)
                .expect("chunk_channel column not found")
                .as_string::<i32>()
                .value(i)
                .to_string();
            let chunk_sequence_id = record_batch.column_by_name(CHUNK_SEQUENCE_ID_COLUMN_NAME)
                .expect("chunk_sequence_id column not found")
                .as_primitive::<Float32Type>()
                .value(i);
            let chunkfile = record_batch.column_by_name(CHUNKFILE_COLUMN_NAME)
                .expect("chunkfile column not found")
                .as_string::<i32>()
                .value(i)
                .to_string();
            let chunk_type = record_batch.column_by_name(CHUNK_TYPE_COLUMN_NAME)
                .expect("chunk_type column not found")
                .as_string::<i32>()
                .value(i);
            let chunk_length = record_batch.column_by_name(CHUNK_LENGTH_COLUMN_NAME)
                .expect("chunk_length column not found")
                .as_primitive::<Float32Type>()
                .value(i);
            let original_file_creation_date = record_batch.column_by_name(FILE_CREATION_DATE_COLUMN_NAME)
                .expect("original_file_creation_date column not found")
                .as_primitive::<TimestampMillisecondType>()
                .value(i);
            let original_file_modified_date = record_batch.column_by_name(FILE_MODIFIED_DATE_COLUMN_NAME)
                .expect("original_file_modified_date column not found")
                .as_primitive::<TimestampMillisecondType>()
                .value(i);
            let original_file_size = record_batch.column_by_name(FILE_SIZE_COLUMN_NAME)
                .expect("original_file_size column not found")
                .as_primitive::<UInt64Type>()
                .value(i);
            let tags_json_str = record_batch.column_by_name(FILE_TAGS_COLUMN_NAME)
                .expect("original_file_tags column not found")
                .as_string::<i32>()
                .value(i);

            let tags: Map<String, Value> = serde_json::from_str(tags_json_str)
                .unwrap_or_else(|_| Map::new());

            ChunkFile {
                original_file: Utf8PathBuf::from(original_file),
                chunk_channel,
                chunk_sequence_id,
                chunkfile: Utf8PathBuf::from(chunkfile),
                chunk_type: string_to_chunk_type(chunk_type),
                chunk_length,
                original_file_creation_date: Utc.timestamp_millis_opt(
                    original_file_creation_date).unwrap(),
                original_file_modified_date: Utc.timestamp_millis_opt(
                    original_file_modified_date).unwrap(),
                original_file_size,
                original_file_tags: tags,
            }
        })
    }
    
    fn attribute_to_column_name(attr: &str) -> &'static str {
        match attr {
            ORIGINAL_FILE_ATTR => ORIGINAL_FILE_COLUMN_NAME,
            CHUNK_CHANNEL_ATTR => CHUNK_CHANNEL_COLUMN_NAME,
            CHUNK_SEQUENCE_ID_ATTR => CHUNK_SEQUENCE_ID_COLUMN_NAME,
            CHUNKFILE_ATTR => CHUNKFILE_COLUMN_NAME,
            CHUNK_TYPE_ATTR => CHUNK_TYPE_COLUMN_NAME,
            CHUNK_LENGTH_ATTR => CHUNK_LENGTH_COLUMN_NAME,
            FILE_CREATION_DATE_ATTR => FILE_CREATION_DATE_COLUMN_NAME,
            FILE_MODIFIED_DATE_ATTR => FILE_MODIFIED_DATE_COLUMN_NAME,
            FILE_SIZE_ATTR => FILE_SIZE_COLUMN_NAME,
            FILE_TAGS_ATTR => FILE_TAGS_COLUMN_NAME,
            _ => panic!("Unknown ChunkFile attribute: {}", attr),
        }
    }
}

impl Filterable for ChunkFile {
    fn filterable_attributes() -> Vec<&'static str> {
        [
            ORIGINAL_FILE_ATTR,
            FILE_CREATION_DATE_ATTR,
            FILE_MODIFIED_DATE_ATTR,
            FILE_SIZE_ATTR,
        ].to_vec()
    }
}

impl FTSData for ChunkFile {
    fn fts_attributes() -> Vec<&'static str> {
        [
            ORIGINAL_FILE_ATTR,
            FILE_TAGS_ATTR,
        ].to_vec()
    }
}

// private methods

fn chunk_type_to_string(ty: ChunkType) -> String {
    match ty {
        ChunkType::Text => "text",
        ChunkType::Image => "image",
        ChunkType::Video => "video",
        ChunkType::Audio => "audio",
    }.to_owned()
}

fn string_to_chunk_type(ty: &str) -> ChunkType {
    match ty {
        "text" => ChunkType::Text,
        "image" => ChunkType::Image,
        "video" => ChunkType::Video,
        "audio" => ChunkType::Audio,
        _ => panic!("invalid chunk_type")
    }
}