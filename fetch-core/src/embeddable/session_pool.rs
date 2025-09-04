use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use ort::session::{builder::GraphOptimizationLevel, Session};
use tokenizers::Tokenizer;

const IMAGE_MODEL_PATH: &str = "models/siglip2-b-16-512/image_embedder.onnx";
const TEXT_MODEL_PATH: &str = "models/siglip2-b-16-512/text_embedder.onnx";
const TOKENIZER_PATH: &str = "models/siglip2-b-16-512/tokenizer.json";

pub static IMAGE_SESSION_POOL: LazyLock<SessionPool> = LazyLock::new(|| {
    create_session_pool(1, PoolType::Image)
});

pub static TEXT_SESSION_POOL: LazyLock<SessionPool> = LazyLock::new(|| {
    create_session_pool(1, PoolType::Text)
});

pub static TEXT_TOKENIZER: LazyLock<Tokenizer> = LazyLock::new(|| {
    Tokenizer::from_file(TOKENIZER_PATH).expect("Error loading tokenizer from file")
});

/// Init function that retrieves indexing resources and then immediately drops them to initialize lazy cells
pub fn init_indexing() {
    IMAGE_SESSION_POOL.get_session();
}

/// Init function that retrieves querying resources and then immediately drops them to initialize lazy cells
pub fn init_querying() {
    TEXT_SESSION_POOL.get_session();
    TEXT_TOKENIZER.encode("hi", false);
}

pub type SessionPool = Arc<Vec<Mutex<Session>>>;

pub trait SessionPoolExt {
    fn get_session(&self) -> MutexGuard<Session>;
}

impl SessionPoolExt for SessionPool {
    fn get_session(&self) -> MutexGuard<Session> {
        for session_mutex in self.iter() {
            if let Ok(session) = session_mutex.try_lock() {
                return session;
            }
        }
        // Fallback to waiting for any available session
        self[0].lock().unwrap()
    }
}

pub enum PoolType {
    Image,
    Text,
}

pub fn create_session_pool(pool_size: u32, pool_type: PoolType) -> SessionPool {
    Arc::new(
        (0..pool_size)
            .map(|_| {
                let session_builder = Session::builder()
                    .expect("Failed to create session builder")
                    .with_optimization_level(GraphOptimizationLevel::Level3)
                    .expect("Failed to set optimization level")
                    .with_intra_threads(4)
                    .expect("Failed to set intra threads");

                let session_result = match pool_type {
                    PoolType::Image => session_builder.commit_from_file(IMAGE_MODEL_PATH),
                    PoolType::Text => session_builder.commit_from_file(TEXT_MODEL_PATH),
                };

                Mutex::new(session_result.expect("Failed to commit model from memory"))
            })
            .collect()
    )
}