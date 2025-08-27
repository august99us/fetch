use std::sync::LazyLock;

use fastembed::{ExecutionProviderDispatch, ImageEmbedding, ImageInitOptionsUserDefined, InitOptionsUserDefined, TextEmbedding, TokenizerFiles, UserDefinedEmbeddingModel, UserDefinedImageEmbeddingModel};
use tokio::task;
use ort::execution_providers::CPUExecutionProvider;

use crate::previewable::{PreviewType, PreviewedFile};

/// Adds the embeddable trait, signifying that a struct or object has data that it can use to
/// create an embedding.
pub trait Embeddable {
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
    #[error("Error while tokenizing query in preparation for embedding")]
    Tokenizing { query: String, #[source] source: anyhow::Error },
    #[error("Error")]
    Unknown { msg: &'static str, #[source] source: anyhow::Error },
}

impl Embeddable for PreviewedFile {
    async fn calculate_embedding(&self) -> Result<Vec<f32>, EmbeddingError> {
        match self.r#type {
            PreviewType::Image => {
                let mut model = get_image_model().map_err(EmbeddingError::Initialization)?;

                // Cloning the path here is necessary because spawn_blocking tasks will not abort if the handle is dropped.
                // They will still complete, therefore they need a valid 'static reference or owned variable. This cloned
                // variable will be dropped as soon as the task completes, so it is a tiny, transient memory overhead.
                let image_path = self.preview_path.clone();
                let result = task::spawn_blocking(move || -> Result<Vec<f32>, EmbeddingError> {
                    // load image
                    let img = image::ImageReader::open(&image_path)
                        .map_err(|e| EmbeddingError::IO { path: image_path.to_string(), source: e.into() })?
                        .decode()
                        .map_err(|e| EmbeddingError::IO { path: image_path.to_string(), source: e.into() })?;

                    // embed image
                    model.embed_images(vec![img])
                        .map(|mut v| v.pop().unwrap())
                        .map_err(|e| EmbeddingError::Calculation { element: image_path.to_string(),
                            step: "Performing image embedding", source: e.into() })
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

impl Embeddable for &str {
    async fn calculate_embedding(&self) -> Result<Vec<f32>, EmbeddingError> {
        let mut model = get_text_model().map_err(EmbeddingError::Initialization)?;

        // clone for async task
        let s = self.to_string();
        let result = task::spawn_blocking(move || -> Result<Vec<f32>, EmbeddingError> {
            model.embed(vec![&s], None)
                .map(|mut v| v.pop().unwrap())
                .map_err(|e| EmbeddingError::Calculation { element: s,
                    step: "Performing text embedding", source: e.into() })
        })
        .await
        .map_err(|e| EmbeddingError::Unknown { msg: "Error while joining embedding blocking task",
            source: e.into() })?;

        result
    }
}

// Private variables and functions
const VISION_MODEL_BYTES: &[u8] = include_bytes!("../artifacts/models/clip-b-32-vision/model.onnx");
const VISION_PREPROCESSOR_CONFIG_BYTES: &[u8] = include_bytes!("../artifacts/models/clip-b-32-vision/preprocessor_config.json");
const TEXT_MODEL_BYTES: &[u8] = include_bytes!("../artifacts/models/clip-b-32-text/model.onnx");
const TEXT_CONFIG_BYTES: &[u8] = include_bytes!("../artifacts/models/clip-b-32-text/config.json");
const TEXT_TOKENIZER_BYTES: &[u8] = include_bytes!("../artifacts/models/clip-b-32-text/tokenizer.json");
const TEXT_TOKENIZER_CONFIG_BYTES: &[u8] = include_bytes!("../artifacts/models/clip-b-32-text/tokenizer_config.json");
const TEXT_SPECIAL_TOKENS_BYTES: &[u8] = include_bytes!("../artifacts/models/clip-b-32-text/special_tokens_map.json");

// Cannot make these static singletons because they need to be mutable
fn get_image_model() -> Result<ImageEmbedding, anyhow::Error> {
    ImageEmbedding::try_new_from_user_defined(
        UserDefinedImageEmbeddingModel::new(
            VISION_MODEL_BYTES.to_vec(),
            VISION_PREPROCESSOR_CONFIG_BYTES.to_vec()),
        ImageInitOptionsUserDefined::default().with_execution_providers(get_execution_providers()),
    )
}

fn get_text_model() -> Result<TextEmbedding, anyhow::Error> {
    TextEmbedding::try_new_from_user_defined(
        UserDefinedEmbeddingModel::new(
            TEXT_MODEL_BYTES.to_vec(),
            TokenizerFiles {
                tokenizer_file: TEXT_TOKENIZER_BYTES.to_vec(),
                config_file: TEXT_CONFIG_BYTES.to_vec(),
                tokenizer_config_file: TEXT_TOKENIZER_CONFIG_BYTES.to_vec(),
                special_tokens_map_file: TEXT_SPECIAL_TOKENS_BYTES.to_vec(),
            }
        ),
        InitOptionsUserDefined::default().with_execution_providers(get_execution_providers()),
    )
}

fn get_execution_providers() -> Vec<ExecutionProviderDispatch> {
    #[cfg(feature = "cuda")]
    {
        use ort::execution_providers::CUDAExecutionProvider;

        vec![Into::<ExecutionProviderDispatch>::into(CUDAExecutionProvider::default()).error_on_failure()]
    }
    #[cfg(feature = "qnn")]
    {
        use ort::execution_providers::QNNExecutionProvider;

        vec![Into::<ExecutionProviderDispatch>::into(QNNExecutionProvider::default()).error_on_failure()]
    }
    #[cfg(not(any(feature = "cuda", feature = "qnn")))]
    {
        vec![CPUExecutionProvider::default().into()]
    }
}