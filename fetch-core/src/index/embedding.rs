
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

pub mod sessions;
pub mod siglip2_image_embedder;