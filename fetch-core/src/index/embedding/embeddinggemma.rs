use std::sync::LazyLock;

use log::debug;
use ndarray::{Array, Axis};
use ort::{inputs, value::TensorRef};
use tokenizers::Tokenizer;
use tokio::{fs, task};

use crate::index::{ChunkFile, ChunkType, embedding::{EmbeddingError, sessions::{SessionPool, SessionPoolExt, create_session_pool, create_tokenizer}}};

impl EmbeddingGemmaEmbeddedChunkFile {
    const VECTOR_LENGTH: u32 = 768;
}

pub struct EmbeddingGemmaEmbeddedChunkFile {
    pub chunkfile: ChunkFile,
    pub embedding: Vec<f32>,
}

pub async fn embed_chunk(chunkfile: ChunkFile) -> Result<EmbeddingGemmaEmbeddedChunkFile, EmbeddingError> {
    if chunkfile.chunk_type != ChunkType::Text {
        return Err(EmbeddingError::InvalidType {
            path: chunkfile.chunkfile.to_string(),
            expected: ChunkType::Text,
            actual: chunkfile.chunk_type
        });
    }

    let text = fs::read_to_string(&chunkfile.chunkfile).await
        .map_err(|e| EmbeddingError::IO { path: chunkfile.chunkfile.to_string(), source: e.into() })?;

    let prompted_text = format!("title: none | text: {text}");

    let embedding = embed_prompted_str(prompted_text).await?;

    Ok(EmbeddingGemmaEmbeddedChunkFile {
        chunkfile,
        embedding,
    })
}

pub async fn embed_query(query: &str) -> Result<Vec<f32>, EmbeddingError> {
    let prompted_query = format!("task: search result | query: {query}");
    embed_prompted_str(prompted_query).await
}

async fn embed_prompted_str(prompt_str: String) -> Result<Vec<f32>, EmbeddingError> {
    let s = prompt_str.to_lowercase();
    let result = task::spawn_blocking(move || -> Result<Vec<f32>, EmbeddingError> {
        let mut model = SESSION_POOL.get_session();
        let tokenizer = &TOKENIZER;
        
        let encoding = tokenizer.encode(s, false)
            .map_err(|e| EmbeddingError::Preprocessing {
                element: format!("Query: {}" , prompt_str),
                step: "tokenizing",
                source: anyhow::anyhow!(e) })?;
        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|n| *n as i64).collect();

        // Pad to 2048 tokens
        let original_length = input_ids.len();
        let mut padded_input_ids = input_ids;
        padded_input_ids.resize(MODEL_INPUT_LENGTH, 0i64);

        // Create attention mask (1 for real tokens, 0 for padding)
        let mut att_mask_vec = vec![1i64; original_length];
        att_mask_vec.resize(MODEL_INPUT_LENGTH, 0i64);

        let input = Array::from_vec(padded_input_ids)
            .insert_axis(Axis(0));
        let att_mask = Array::from_vec(att_mask_vec)
            .insert_axis(Axis(0));

        let result = model.run(inputs![
                "input_ids" => TensorRef::from_array_view(&input)
                    .map_err(|e| EmbeddingError::Preprocessing { 
                        element: format!("Query: {}" , prompt_str),
                        step: "Converting to tensor", 
                        source: e.into(),
                    })?,
                "attention_mask" => TensorRef::from_array_view(&att_mask)
                    .map_err(|e| EmbeddingError::Preprocessing { 
                        element: format!("Query: {}" , prompt_str),
                        step: "Converting to tensor", 
                        source: e.into(),
                    })?,
            ])
            .map_err(|e| EmbeddingError::Calculation {
                element: format!("Query: {}" , prompt_str),
                step: "Performing text embedding", source: e.into(),
            })?
            .get("sentence_embedding")
            .expect("model should place output in 'sentence_embedding' key")
            .try_extract_array::<f32>()
            .map_err(|e| EmbeddingError::Unknown {
                msg: "Error while extracting array from output as f32",
                source: e.into(),
            })?
            .into_owned()
            .into_shape_with_order((EmbeddingGemmaEmbeddedChunkFile::VECTOR_LENGTH as usize,))
            .expect("Model should return a (1, 768) shaped array which should be able to be reshaped into a vector")
            .to_vec();
        
        Ok(result)
    })
    .await
    .map_err(|e| EmbeddingError::Unknown { msg: "Error while joining embedding blocking task",
        source: e.into() })?;

    result
}

/// Init function that retrieves querying resources and then immediately drops them to initialize lazy cells
/// 
/// sessions::init_model_resource_directory must be called before this function or all models will be initialized
/// from a binary relative models/ path
pub fn init() {
    // Instantiate and instantly drop the mutex guard to load the sessions
    let _guard = SESSION_POOL.get_session();
}

pub use integrations::*;

// Private functions and variables

const MODEL_INPUT_LENGTH: usize = 2048;

const MODEL_PATH: &str = "embeddinggemma-300m/model.onnx";
const TOKENIZER_PATH: &str = "embeddinggemma-300m/tokenizer.json";

static SESSION_POOL: LazyLock<SessionPool> = LazyLock::new(|| {
    debug!("Initializing text embedding resources for EmbeddingGemma Embedder");
    create_session_pool(1, MODEL_PATH.into())
});

static TOKENIZER: LazyLock<Tokenizer> = LazyLock::new(|| {
    debug!("Initializing text tokenizer resources for EmbeddingGemma Embedder");
    create_tokenizer(TOKENIZER_PATH.into())
});

mod integrations;