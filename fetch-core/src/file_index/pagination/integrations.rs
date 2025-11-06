use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use arrow::array::{AsArray, StringBuilder, TimestampMillisecondBuilder, UInt32Builder};
use arrow::datatypes::{TimestampMillisecondType, UInt32Type};
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use camino::Utf8PathBuf;
use chrono::{TimeZone, Utc};

use crate::file_index::pagination::{AggregateFileScore, QueryCursor};
use crate::store::lancedb::{ArrowData, RowBuilder};
use crate::store::{Filterable, KeyedSequencedData};

// ===========================
// Attribute and Column Names
// ===========================
pub const CURSOR_ID_ATTR: &str = "id";
pub const AGGREGATE_SCORES_ATTR: &str = "aggregate_scores";
pub const CURR_OFFSET_ATTR: &str = "curr_offset";
pub const TTL_ATTR: &str = "ttl";

const CURSOR_ID_COLUMN_NAME: &str = "cursor_id";
const AGGREGATE_SCORES_COLUMN_NAME: &str = "aggregate_scores";
const CURR_OFFSET_COLUMN_NAME: &str = "curr_offset";
const TTL_COLUMN_NAME: &str = "ttl";

// ===========================
// Schema Definition
// ===========================
static CURSOR_ID_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(
        CURSOR_ID_COLUMN_NAME,
        DataType::Utf8,
        false,
    ))
});

static AGGREGATE_SCORES_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(
        AGGREGATE_SCORES_COLUMN_NAME,
        DataType::Utf8, // JSON serialized as string
        false,
    ))
});

static CURR_OFFSET_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(
        CURR_OFFSET_COLUMN_NAME,
        DataType::UInt32,
        false,
    ))
});

static TTL_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(
        TTL_COLUMN_NAME,
        DataType::Timestamp(TimeUnit::Millisecond, None),
        false,
    ))
});

static CURSOR_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    Schema::new(vec![
        Arc::clone(&CURSOR_ID_FIELD),
        Arc::clone(&AGGREGATE_SCORES_FIELD),
        Arc::clone(&CURR_OFFSET_FIELD),
        Arc::clone(&TTL_FIELD),
    ])
});

// ===========================
// KeyedSequencedData Implementation
// ===========================
impl KeyedSequencedData<String> for QueryCursor {
    fn get_key(&self) -> String {
        self.id.clone()
    }

    fn get_sequence_num(&self) -> u64 {
        // Use current offset as sequence number for pagination ordering
        self.curr_offset as u64
    }
}

// ===========================
// ArrowData RowBuilder
// ===========================
pub struct CursorRowBuilder {
    cursor_id: StringBuilder,
    aggregate_scores: StringBuilder,
    curr_offset: UInt32Builder,
    ttl: TimestampMillisecondBuilder,
}

impl CursorRowBuilder {
    fn new() -> Self {
        Self {
            cursor_id: StringBuilder::new(),
            aggregate_scores: StringBuilder::new(),
            curr_offset: UInt32Builder::new(),
            ttl: TimestampMillisecondBuilder::new(),
        }
    }
}

impl RowBuilder<QueryCursor> for CursorRowBuilder {
    fn append(&mut self, row: QueryCursor) {
        self.cursor_id.append_value(&row.id);

        // Serialize aggregate_scores as JSON
        let scores_json = serde_json::to_string(&row.aggregate_scores)
            .unwrap_or_else(|_| "{}".to_string());
        self.aggregate_scores.append_value(&scores_json);

        self.curr_offset.append_value(row.curr_offset);
        self.ttl.append_value(row.ttl.timestamp_millis());
    }

    fn finish(mut self) -> Vec<(Arc<Field>, ArrayRef)> {
        vec![
            (Arc::clone(&CURSOR_ID_FIELD), Arc::new(self.cursor_id.finish())),
            (
                Arc::clone(&AGGREGATE_SCORES_FIELD),
                Arc::new(self.aggregate_scores.finish()),
            ),
            (
                Arc::clone(&CURR_OFFSET_FIELD),
                Arc::new(self.curr_offset.finish()),
            ),
            (Arc::clone(&TTL_FIELD), Arc::new(self.ttl.finish())),
        ]
    }
}

// ===========================
// ArrowData Implementation
// ===========================
impl ArrowData for QueryCursor {
    type RowBuilder = CursorRowBuilder;

    fn schema() -> Schema {
        CURSOR_SCHEMA.clone()
    }

    fn row_builder() -> Self::RowBuilder {
        CursorRowBuilder::new()
    }

    fn attribute_to_column_name(attr: &str) -> &'static str {
        match attr {
            CURSOR_ID_ATTR => CURSOR_ID_COLUMN_NAME,
            AGGREGATE_SCORES_ATTR => AGGREGATE_SCORES_COLUMN_NAME,
            CURR_OFFSET_ATTR => CURR_OFFSET_COLUMN_NAME,
            TTL_ATTR => TTL_COLUMN_NAME,
            _ => panic!("Unknown Cursor attribute: {}", attr),
        }
    }

    fn batch_to_iter(record_batch: RecordBatch) -> impl IntoIterator<Item = Self> {
        let num_rows = record_batch.num_rows();

        // Move record_batch into the iterator by capturing it in the closure
        // This is necessary because the serde_json::from_str function itself captures the
        // lifetime of the &str it is deserializing from, so the original owner of the data
        // (record batch -> &ByteArray -> &json str) needs to life for as long as the iterator
        (0..num_rows).map(move |i| {
            // Extract values from the batch on each iteration
            let cursor_id = record_batch
                .column_by_name(CURSOR_ID_COLUMN_NAME)
                .expect("cursor_id column not found")
                .as_string::<i32>()
                .value(i)
                .to_string();

            let aggregate_scores_json = record_batch
                .column_by_name(AGGREGATE_SCORES_COLUMN_NAME)
                .expect("aggregate_scores column not found")
                .as_string::<i32>()
                .value(i);

            let curr_offset = record_batch
                .column_by_name(CURR_OFFSET_COLUMN_NAME)
                .expect("curr_offset column not found")
                .as_primitive::<UInt32Type>()
                .value(i);

            let ttl_value = record_batch
                .column_by_name(TTL_COLUMN_NAME)
                .expect("ttl column not found")
                .as_primitive::<TimestampMillisecondType>()
                .value(i);

            // Deserialize aggregate_scores from JSON
            let scores: HashMap<Utf8PathBuf, AggregateFileScore> =
                serde_json::from_str(aggregate_scores_json)
                    .unwrap_or_else(|_| HashMap::new());

            QueryCursor {
                id: cursor_id,
                aggregate_scores: scores,
                curr_offset,
                ttl: Utc.timestamp_millis_opt(ttl_value).unwrap(),
            }
        })
    }
}

// ===========================
// Filterable Implementation
// ===========================
impl Filterable for QueryCursor {
    fn filterable_attributes() -> Vec<&'static str> {
        vec![TTL_ATTR]
    }
}