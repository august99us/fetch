use std::sync::MutexGuard;

use camino::{Utf8Path, Utf8PathBuf};
use chrono::{DateTime, Utc};
use image::GenericImageView;
use ndarray::{Array, Axis};
use ort::{inputs, session::Session, value::TensorRef};
use serde_json::{Map, Value};
use tokio::task;

use crate::{indexing::{session_pool::{IMAGE_SESSION_POOL, TEXT_SESSION_POOL, TEXT_TOKENIZER}, store::KeyedSequencedData}, previewable::{PreviewType, PreviewedFile}};
use session_pool::SessionPoolExt;

pub trait IndexProvider {
    fn provides_indexing_for_extensions() -> Vec<String>;
    fn index(path: &Utf8Path) -> Result<(), anyhow::Error>;
    fn clear(path: &Utf8Path) -> Result<(), anyhow::Error>;
    fn query(str: &str) -> Result<Vec<Utf8PathBuf>, anyhow::Error>;
}

pub trait ChunkingIndexProvider {
    fn query_chunks(str: &str) -> Result<Vec<ChunkFile>, anyhow::Error>;
}

pub struct ChunkFile {
    // Composite key
    pub original_file: Utf8PathBuf,
    pub chunk_channel: String,
    pub chunk_sequence_id: f32,
    // Other data pieces
    pub chunkfile: Utf8PathBuf,
    pub chunk_length: f32,
    pub original_file_creation_date: DateTime<Utc>,
    pub original_file_modified_date: DateTime<Utc>,
    pub original_file_size: u64,
    pub original_file_tags: Map<String, Value>,
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

/// Adds the embeddable trait, signifying that a struct or object has data that it can use to
/// create an embedding.
pub trait Indexable {
    /// Calculates the embedding for the presented data in the objects using the Embedder passed in the
    /// arguments. Embedder model should support both image and text embeddings.
    async fn calculate_embedding(&self) -> Result<Vec<f32>, EmbeddingError>;
}

#[derive(thiserror::Error, Debug)]
pub enum EmbeddingError {
    #[error("Error during intialization of model and tokenizer for embedding")]
    Initialization (#[source] anyhow::Error),
    #[error("Error interacting with file")]
    IO { path: String, #[source] source: anyhow::Error },
    #[error("Error while performing neural network calculations with file")]
    Calculation { element: String, step: &'static str, #[source] source: anyhow::Error },
    #[error("Error while preprocessing data in preparation for embedding")]
    Preprocessing { element: String, step: &'static str, #[source] source: anyhow::Error },
    #[error("Error")]
    Unknown { msg: &'static str, #[source] source: anyhow::Error },
}

impl Indexable for PreviewedFile {
    async fn calculate_embedding(&self) -> Result<Vec<f32>, EmbeddingError> {
        match self.r#type {
            PreviewType::Image => {
                // Cloning the path here is necessary for the blocking task
                let image_path = self.preview_path.clone();
                let result = task::spawn_blocking(move || -> Result<Vec<f32>, EmbeddingError> {
                    // Get session from pool inside the blocking task
                    let mut session = get_image_session();
                    
                    // load image
                    let img = image::ImageReader::open(&image_path)
                        .map_err(|e| EmbeddingError::IO { path: image_path.to_string(), source: e.into() })?
                        .decode()
                        .map_err(|e| EmbeddingError::IO { path: image_path.to_string(), source: e.into() })?;
                    
                    let resized_img = img.resize_exact(512, 512, image::imageops::FilterType::Triangle);
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
                    let result = session.run(inputs![
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
                        .into_shape_with_order((768,))
                        .expect("Model should return a (1, 768) shaped array which should be able to be reshaped into a vector")
                        .to_vec();

                    Ok(result)
                })
                .await
                .map_err(|e| EmbeddingError::Unknown { msg: "Error while joining embedding blocking task",
                    source: e.into() })?;

                result
            },
            _ => todo!(),
        }
    }
}

impl Indexable for &str {
    async fn calculate_embedding(&self) -> Result<Vec<f32>, EmbeddingError> {
        // clone for async task, lower for siglip2
        let query_copy = self.to_string();
        let s = self.to_lowercase();
        let result = task::spawn_blocking(move || -> Result<Vec<f32>, EmbeddingError> {
            let mut model = get_text_session();
            let tokenizer = &TEXT_TOKENIZER;
            
            let encoding = tokenizer.encode(s, false)
                .map_err(|e| EmbeddingError::Preprocessing { 
                    element: format!("Query: {}" , query_copy),
                    step: "tokenizing",
                    source: anyhow::anyhow!(e) })?;
            let input_ids = encoding.get_ids().into_iter().map(|n| *n as i64).collect();
            
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
                .into_shape_with_order((768,))
                .expect("Model should return a (1, 768) shaped array which should be able to be reshaped into a vector")
                .to_vec();
            
            Ok(result)
        })
        .await
        .map_err(|e| EmbeddingError::Unknown { msg: "Error while joining embedding blocking task",
            source: e.into() })?;

        result
    }
}

pub mod basic_image_index_provider;
pub mod chunker;
pub mod embedder;
pub mod session_pool;
pub mod store;

// Private variables and functions

fn get_image_session() -> MutexGuard<'static, Session> {
    IMAGE_SESSION_POOL.get_session()
}

fn get_text_session() -> MutexGuard<'static, Session> {
    TEXT_SESSION_POOL.get_session()
}

mod integrations;
pub use integrations::*;