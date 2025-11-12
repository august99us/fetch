use std::sync::LazyLock;

use image::{GenericImageView, imageops::FilterType};
use log::debug;
use ndarray::{Array, Axis};
use ort::{inputs, value::TensorRef};
use tokenizers::Tokenizer;
use tokio::task;

use crate::index::{ChunkFile, ChunkType, embedding::{EmbeddingError, sessions::{SessionPool, SessionPoolExt, create_session_pool, create_tokenizer}}};

impl Siglip2EmbeddedChunkFile {
    const VECTOR_LENGTH: u32 = 768;
}

pub struct Siglip2EmbeddedChunkFile {
    pub chunkfile: ChunkFile,
    pub embedding: Vec<f32>,
}

pub async fn embed_chunk(chunkfile: ChunkFile) -> Result<Siglip2EmbeddedChunkFile, EmbeddingError> {
    if chunkfile.chunk_type != ChunkType::Image {
        return Err(EmbeddingError::InvalidType {
            path: chunkfile.chunkfile.to_string(),
            expected: ChunkType::Image,
            actual: chunkfile.chunk_type
        });
    }

    let image_path = chunkfile.chunkfile.clone();
    let vector = task::spawn_blocking(move || -> Result<Vec<f32>, EmbeddingError> {
        // Get session from pool inside the blocking task
        let mut model = IMAGE_SESSION_POOL.get_session();
        
        // load image
        let img = image::ImageReader::open(&image_path)
            .map_err(|e| EmbeddingError::IO { path: image_path.to_string(), source: e.into() })?
            .decode()
            .map_err(|e| EmbeddingError::IO { path: image_path.to_string(), source: e.into() })?;
        
        let resized_img = img.resize_exact(512, 512, FilterType::Triangle);
        let mut input = Array::zeros((1, 3, 512, 512));
        for pixel in resized_img.pixels() {
            let x = pixel.0 as _;
            let y = pixel.1 as _;
            let [r, g, b, _] = pixel.2.0;
            input[[0, 0, y, x]] = (r as f32) / 255.;
            input[[0, 1, y, x]] = (g as f32) / 255.;
            input[[0, 2, y, x]] = (b as f32) / 255.;
        }

        // embed image
        let result = model.run(inputs![
                "input" => TensorRef::from_array_view(&input)
                    .map_err(|e| EmbeddingError::Preprocessing { 
                        element: image_path.to_string(), 
                        step: "Converting to tensor", 
                        source: e.into(),
                    })?
            ])
            .map_err(|e| EmbeddingError::Calculation { element: image_path.to_string(),
                step: "Performing image embedding", source: e.into() })?
            .get("output")
            .expect("model should place output in 'output' key")
            .try_extract_array::<f32>()
            .map_err(|e| EmbeddingError::Unknown {
                msg: "Error while extracting array from output as f32",
                source: e.into(),
            })?
            .into_owned()
            .into_shape_with_order((Siglip2EmbeddedChunkFile::VECTOR_LENGTH as usize,))
            .expect("Model should return a (1, 768) shaped array which should be able to be reshaped into a vector")
            .to_vec();

        Ok(result)
    })
    .await
    .map_err(|e| EmbeddingError::Unknown { msg: "Error while joining embedding blocking task",
        source: e.into() })??;

    Ok(Siglip2EmbeddedChunkFile {
        chunkfile,
        embedding: vector,
    })
}

pub async fn embed_query(query: &str) -> Result<Vec<f32>, EmbeddingError> {
    let query_copy = query.to_string();
    let s = query.to_lowercase();
    let result = task::spawn_blocking(move || -> Result<Vec<f32>, EmbeddingError> {
        let mut model = TEXT_SESSION_POOL.get_session();
        let tokenizer = &TEXT_TOKENIZER;
        
        let encoding = tokenizer.encode(s, false)
            .map_err(|e| EmbeddingError::Preprocessing { 
                element: format!("Query: {}" , query_copy),
                step: "tokenizing",
                source: anyhow::anyhow!(e) })?;
        let input_ids = encoding.get_ids().iter().map(|n| *n as i64).collect();

        let input = Array::from_vec(input_ids)
            .insert_axis(Axis(0));

        let result = model.run(inputs![
                "input" => TensorRef::from_array_view(&input)
                    .map_err(|e| EmbeddingError::Preprocessing { 
                        element: format!("Query: {}" , query_copy),
                        step: "Converting to tensor", 
                        source: e.into(),
                    })?
            ])
            .map_err(|e| EmbeddingError::Calculation {
                element: format!("Query: {}" , query_copy),
                step: "Performing text embedding", source: e.into()
            })?
            .get("output")
            .expect("model should place output in 'output' key")
            .try_extract_array::<f32>()
            .map_err(|e| EmbeddingError::Unknown {
                msg: "Error while extracting array from output as f32",
                source: e.into(),
            })?
            .into_owned()
            .into_shape_with_order((Siglip2EmbeddedChunkFile::VECTOR_LENGTH as usize,))
            .expect("Model should return a (1, 768) shaped array which should be able to be reshaped into a vector")
            .to_vec();
        
        Ok(result)
    })
    .await
    .map_err(|e| EmbeddingError::Unknown { msg: "Error while joining embedding blocking task",
        source: e.into() })?;

    result
}

/// Init function that retrieves indexing resources and then immediately drops them to initialize lazy cells
/// 
/// sessions::init_model_resource_directory must be called before this function or all models will be initialized
/// from a binary relative models/ path
pub fn init_indexing() {
    LazyLock::force(&IMAGE_SESSION_POOL);
}

/// Init function that retrieves querying resources and then immediately drops them to initialize lazy cells
/// 
/// sessions::init_model_resource_directory must be called before this function or all models will be initialized
/// from a binary relative models/ path
pub fn init_querying() {
    LazyLock::force(&TEXT_SESSION_POOL);
}

pub use integrations::*;

// Private functions and variables

const IMAGE_MODEL_PATH: &str = "siglip2-base-patch16-512/image_embedder.onnx";
const TEXT_MODEL_PATH: &str = "siglip2-base-patch16-512/text_embedder.onnx";
const TOKENIZER_PATH: &str = "siglip2-base-patch16-512/tokenizer.json";

static IMAGE_SESSION_POOL: LazyLock<SessionPool> = LazyLock::new(|| {
    debug!("Initializing image embedding resources for Siglip2 Embedder");
    create_session_pool(1, IMAGE_MODEL_PATH.into())
});

static TEXT_SESSION_POOL: LazyLock<SessionPool> = LazyLock::new(|| {
    debug!("Initializing text embedding resources for Siglip2 Embedder");
    create_session_pool(1, TEXT_MODEL_PATH.into())
});

static TEXT_TOKENIZER: LazyLock<Tokenizer> = LazyLock::new(|| {
    debug!("Initializing text tokenizer resources for Siglip2 Embedder");
    create_tokenizer(TOKENIZER_PATH.into())
});

mod integrations;