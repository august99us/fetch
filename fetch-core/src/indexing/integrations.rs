use std::sync::{Arc, LazyLock};
use arrow::array::{StringBuilder, Float32Builder, UInt64Builder, TimestampNanosecondBuilder, AsArray};
use arrow_array::{RecordBatch, ArrayRef};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use camino::Utf8PathBuf;
use chrono::{Utc, TimeZone};
use serde_json::Value;
use serde_json::Map;

use crate::indexing::store::lancedb::ArrowData;
use crate::indexing::store::lancedb::RowBuilder;
use crate::indexing::store::Filterable;
use crate::indexing::ChunkFile;

// Chunkfile ArrowData integrations

// Attribute names (field names on the ChunkFile struct)
const ORIGINAL_FILE_ATTR: &str = "original_file";
const CHUNK_CHANNEL_ATTR: &str = "chunk_channel";
const CHUNK_SEQUENCE_ID_ATTR: &str = "chunk_sequence_id";
const CHUNKFILE_ATTR: &str = "chunkfile";
const CHUNK_LENGTH_ATTR: &str = "chunk_length";
const FILE_CREATION_DATE_ATTR: &str = "original_file_creation_date";
const FILE_MODIFIED_DATE_ATTR: &str = "original_file_modified_date";
const FILE_SIZE_ATTR: &str = "original_file_size";
const FILE_TAGS_ATTR: &str = "original_file_tags";

// Column names (Arrow schema column names)
const ORIGINAL_FILE_COLUMN_NAME: &str = "original_file";
const CHUNK_CHANNEL_COLUMN_NAME: &str = "chunk_channel";
const CHUNK_SEQUENCE_ID_COLUMN_NAME: &str = "chunk_sequence_id";
const CHUNKFILE_COLUMN_NAME: &str = "chunkfile";
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
static CHUNK_LENGTH_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(CHUNK_LENGTH_COLUMN_NAME, DataType::Float32, false))
});
static FILE_CREATION_DATE_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(FILE_CREATION_DATE_COLUMN_NAME, DataType::Timestamp(TimeUnit::Nanosecond, Some("UTC".into())), false))
});
static FILE_MODIFIED_DATE_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(FILE_MODIFIED_DATE_COLUMN_NAME, DataType::Timestamp(TimeUnit::Nanosecond, Some("UTC".into())), false))
});
static FILE_SIZE_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(FILE_SIZE_COLUMN_NAME, DataType::UInt64, false))
});
static FILE_TAGS_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(FILE_TAGS_COLUMN_NAME, DataType::Utf8, false))
});

static CHUNKFILE_SCHEMA: LazyLock<Arc<Schema>> = LazyLock::new(|| {
    Arc::new(Schema::new(vec![
        ORIGINAL_FILE_FIELD,
        CHUNK_CHANNEL_FIELD,
        CHUNK_SEQUENCE_ID_FIELD,
        CHUNKFILE_FIELD,
        CHUNK_LENGTH_FIELD,
        FILE_CREATION_DATE_FIELD,
        FILE_MODIFIED_DATE_FIELD,
        FILE_SIZE_FIELD,
        FILE_TAGS_FIELD,
    ]))
});

pub struct ChunkFileRowBuilder {
    original_file: StringBuilder,
    chunk_channel: StringBuilder,
    chunk_sequence_id: Float32Builder,
    chunkfile: StringBuilder,
    chunk_length: Float32Builder,
    original_file_creation_date: TimestampNanosecondBuilder,
    original_file_modified_date: TimestampNanosecondBuilder,
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
            chunk_length: Float32Builder::new(),
            original_file_creation_date: TimestampNanosecondBuilder::new().with_timezone("UTC"),
            original_file_modified_date: TimestampNanosecondBuilder::new().with_timezone("UTC"),
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
        self.chunk_length.append_value(row.chunk_length);
        self.original_file_creation_date.append_value(row.original_file_creation_date.timestamp_nanos_opt().unwrap());
        self.original_file_modified_date.append_value(row.original_file_modified_date.timestamp_nanos_opt().unwrap());
        self.original_file_size.append_value(row.original_file_size);

        // Serialize tags as JSON string
        let tags_json = serde_json::to_string(&row.original_file_tags).unwrap_or_else(|_| "{}".to_string());
        self.original_file_tags.append_value(&tags_json);
    }

    fn finish(mut self) -> Vec<(Arc<Field>, ArrayRef)> {
        vec![
            (ORIGINAL_FILE_FIELD, Arc::new(self.original_file.finish())),
            (CHUNK_CHANNEL_FIELD, Arc::new(self.chunk_channel.finish())),
            (CHUNK_SEQUENCE_ID_FIELD, Arc::new(self.chunk_sequence_id.finish())),
            (CHUNKFILE_FIELD, Arc::new(self.chunkfile.finish())),
            (CHUNK_LENGTH_FIELD, Arc::new(self.chunk_length.finish())),
            (FILE_CREATION_DATE_FIELD, Arc::new(self.original_file_creation_date.finish())),
            (FILE_MODIFIED_DATE_FIELD, Arc::new(self.original_file_modified_date.finish())),
            (FILE_SIZE_FIELD, Arc::new(self.original_file_size.finish())),
            (FILE_TAGS_FIELD, Arc::new(self.original_file_tags.finish())),
        ]
    }
}

impl ArrowData for ChunkFile {
    type RowBuilder = ChunkFileRowBuilder;

    fn schema() -> Arc<Schema> {
        CHUNKFILE_SCHEMA
    }

    fn row_builder() -> Self::RowBuilder {
        ChunkFileRowBuilder::new()
    }

    fn batch_to_iter(record_batch: RecordBatch) -> impl IntoIterator<Item = Self> {
        let original_file = record_batch.column_by_name(ORIGINAL_FILE_COLUMN_NAME)
            .expect("original_file column not found")
            .as_string::<i32>();
        let chunk_channel = record_batch.column_by_name(CHUNK_CHANNEL_COLUMN_NAME)
            .expect("chunk_channel column not found")
            .as_string::<i32>();
        let chunk_sequence_id = record_batch.column_by_name(CHUNK_SEQUENCE_ID_COLUMN_NAME)
            .expect("chunk_sequence_id column not found")
            .as_primitive::<arrow::datatypes::Float32Type>();
        let chunkfile = record_batch.column_by_name(CHUNKFILE_COLUMN_NAME)
            .expect("chunkfile column not found")
            .as_string::<i32>();
        let chunk_length = record_batch.column_by_name(CHUNK_LENGTH_COLUMN_NAME)
            .expect("chunk_length column not found")
            .as_primitive::<arrow::datatypes::Float32Type>();
        let original_file_creation_date = record_batch.column_by_name(FILE_CREATION_DATE_COLUMN_NAME)
            .expect("original_file_creation_date column not found")
            .as_primitive::<arrow::datatypes::TimestampNanosecondType>();
        let original_file_modified_date = record_batch.column_by_name(FILE_MODIFIED_DATE_COLUMN_NAME)
            .expect("original_file_modified_date column not found")
            .as_primitive::<arrow::datatypes::TimestampNanosecondType>();
        let original_file_size = record_batch.column_by_name(FILE_SIZE_COLUMN_NAME)
            .expect("original_file_size column not found")
            .as_primitive::<arrow::datatypes::UInt64Type>();
        let original_file_tags = record_batch.column_by_name(FILE_TAGS_COLUMN_NAME)
            .expect("original_file_tags column not found")
            .as_string::<i32>();

        (0..record_batch.num_rows()).map(move |i| {
            let tags_json_str = original_file_tags.value(i);
            let tags: Map<String, Value> = serde_json::from_str(tags_json_str)
                .unwrap_or_else(|_| Map::new());

            ChunkFile {
                original_file: Utf8PathBuf::from(original_file.value(i)),
                chunk_channel: chunk_channel.value(i).to_string(),
                chunk_sequence_id: chunk_sequence_id.value(i),
                chunkfile: Utf8PathBuf::from(chunkfile.value(i)),
                chunk_length: chunk_length.value(i),
                original_file_creation_date: Utc.timestamp_nanos(original_file_creation_date.value(i)),
                original_file_modified_date: Utc.timestamp_nanos(original_file_modified_date.value(i)),
                original_file_size: original_file_size.value(i),
                original_file_tags: tags,
            }
        })
    }
    
    fn attribute_to_column_name(attr: &'static str) -> &'static str {
        match attr {
            ORIGINAL_FILE_ATTR => ORIGINAL_FILE_COLUMN_NAME,
            CHUNK_CHANNEL_ATTR => CHUNK_CHANNEL_COLUMN_NAME,
            CHUNK_SEQUENCE_ID_ATTR => CHUNK_SEQUENCE_ID_COLUMN_NAME,
            CHUNKFILE_ATTR => CHUNKFILE_COLUMN_NAME,
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

}