use crate::index::ChunkType;

#[derive(thiserror::Error, Debug)]
pub enum EmbeddingError {
    #[error("Invalid chunk type provided to model. Expected {expected:?} but got {actual:?} for file at path: {path}")]
    InvalidType { path: String, expected: ChunkType, actual: ChunkType },
    #[error("Error during intialization of model and tokenizer for embedding")]
    Initialization (#[source] anyhow::Error),
    #[error("Error interacting with file at {path}")]
    IO { path: String, #[source] source: anyhow::Error },
    #[error("Error while performing neural network calculations with file: {element} at step: {step}")]
    Calculation { element: String, step: &'static str, #[source] source: anyhow::Error },
    #[error("Error while preprocessing data in preparation for embedding: {element} at step: {step}")]
    Preprocessing { element: String, step: &'static str, #[source] source: anyhow::Error },
    #[error("Error: {msg}")]
    Unknown { msg: &'static str, #[source] source: anyhow::Error },
}

pub mod sessions;

// model modules
pub mod embeddinggemma;
pub mod siglip2;