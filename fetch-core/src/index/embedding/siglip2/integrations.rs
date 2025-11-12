use std::sync::{Arc, LazyLock};

use arrow::array::{AsArray, FixedSizeListBuilder, Float32Builder};
use arrow::datatypes::Float32Type;
use arrow_array::{ArrayRef, FixedSizeListArray, RecordBatch};
use arrow_schema::{DataType, Field, Schema};

use crate::index::{ChunkFile, embedding::siglip2::Siglip2EmbeddedChunkFile};
use crate::store::{FTSData, Filterable, lancedb::{ArrowData, RowBuilder}, KeyedSequencedData, VectorData};

impl Siglip2EmbeddedChunkFile {
    const VECTOR_ATTRIBUTE_NAME: &str = "embedding";
    const VECTOR_COLUMN_NAME: &str = "embedding";
}

static VECTOR_FIELD: LazyLock<Arc<Field>> = LazyLock::new(|| {
    Arc::new(Field::new(
        Siglip2EmbeddedChunkFile::VECTOR_COLUMN_NAME,
        DataType::FixedSizeList(
            // This should not be nullable=true but i have not been able to get lancedb
            // to accept nullable=false. it converts nullable false -> true quietly every
            // time.
            Arc::new(Field::new("item", DataType::Float32, true)),
            Siglip2EmbeddedChunkFile::VECTOR_LENGTH.try_into().unwrap(),
        ),
        false,
    ))
});

pub struct Siglip2EmbeddedChunkFileRowBuilder {
    chunkfile_builder: <ChunkFile as ArrowData>::RowBuilder,
    vector_builder: FixedSizeListBuilder<Float32Builder>,
}

impl Default for Siglip2EmbeddedChunkFileRowBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Siglip2EmbeddedChunkFileRowBuilder {
    pub fn new() -> Self {
        Self {
            chunkfile_builder: ChunkFile::row_builder(),
            vector_builder: FixedSizeListBuilder::new(Float32Builder::new(),
                Siglip2EmbeddedChunkFile::VECTOR_LENGTH.try_into().unwrap()),
        }
    }
}

impl RowBuilder<Siglip2EmbeddedChunkFile> for Siglip2EmbeddedChunkFileRowBuilder {
    fn append(&mut self, row: Siglip2EmbeddedChunkFile) {
        // Delegate ChunkFile fields to the ChunkFile builder
        self.chunkfile_builder.append(row.chunkfile);

        // Append vector data
        for value in row.embedding {
            self.vector_builder.values().append_value(value);
        }
        self.vector_builder.append(true);
    }

    fn finish(mut self) -> Vec<(Arc<Field>, ArrayRef)> {
        let mut columns = self.chunkfile_builder.finish();
        columns.push((VECTOR_FIELD.clone(), 
            Arc::new(self.vector_builder.finish()) as ArrayRef));
        columns
    }
}

impl ArrowData for Siglip2EmbeddedChunkFile {
    type RowBuilder = Siglip2EmbeddedChunkFileRowBuilder;

    fn schema() -> Schema {
        // Construct schema dynamically by combining ChunkFile schema with vector field
        let chunkfile_schema = ChunkFile::schema();
        let extended_schema = Schema::new(vec![VECTOR_FIELD.clone()]);
        Schema::try_merge([chunkfile_schema, extended_schema])
            .unwrap_or_else(|_e| panic!("Siglip2EmbeddedChunkFile extended schema \
                could not be merged with ChunkFile schema"))
    }

    fn row_builder() -> Self::RowBuilder {
        Siglip2EmbeddedChunkFileRowBuilder::new()
    }

    fn attribute_to_column_name(attr: &str) -> &'static str {
        // Delegate to ChunkFile for its attributes, handle "embedding" ourselves
        if attr == Siglip2EmbeddedChunkFile::VECTOR_ATTRIBUTE_NAME {
            Siglip2EmbeddedChunkFile::VECTOR_COLUMN_NAME
        } else {
            ChunkFile::attribute_to_column_name(attr)
        }
    }

    fn batch_to_iter(record_batch: RecordBatch) -> impl IntoIterator<Item = Self> {
        // Extract vector column
        let vector_column = record_batch.column_by_name(Siglip2EmbeddedChunkFile::VECTOR_COLUMN_NAME)
            .expect("embedding column should exist")
            .as_any().downcast_ref::<FixedSizeListArray>()
            .expect("Embedding column could not be cast to FixedSizeListArray")
            .iter()
                .map(|a| a.expect("vector should exist")
                    .as_primitive::<Float32Type>()
                    .values()
                    .to_vec())
            .collect::<Vec<Vec<f32>>>();

        // Get ChunkFile iterator
        let chunkfile_iter = ChunkFile::batch_to_iter(record_batch).into_iter();

        // Combine ChunkFile with vectors
        chunkfile_iter.zip(vector_column)
            .map(|(chunkfile, embedding)| Siglip2EmbeddedChunkFile {
                chunkfile,
                embedding,
            })
    }
}

impl VectorData for Siglip2EmbeddedChunkFile {
    fn get_vector(&self) -> &[f32] {
        &self.embedding
    }

    fn vector_attribute() -> &'static str {
        Siglip2EmbeddedChunkFile::VECTOR_ATTRIBUTE_NAME
    }

    fn vector_length() -> u32 {
        Siglip2EmbeddedChunkFile::VECTOR_LENGTH
    }
}

impl KeyedSequencedData<String> for Siglip2EmbeddedChunkFile {
    fn get_key(&self) -> String {
        // Delegate to ChunkFile's implementation
        self.chunkfile.get_key()
    }

    fn get_sequence_num(&self) -> u64 {
        // Delegate to ChunkFile's implementation
        self.chunkfile.get_sequence_num()
    }
}

impl Filterable for Siglip2EmbeddedChunkFile {
    fn filterable_attributes() -> Vec<&'static str> {
        ChunkFile::filterable_attributes()
    }
}

impl FTSData for Siglip2EmbeddedChunkFile {
    fn fts_attributes() -> Vec<&'static str> {
        ChunkFile::fts_attributes()
    }
}