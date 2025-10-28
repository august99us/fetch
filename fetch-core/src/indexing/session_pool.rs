use std::sync::{Arc, LazyLock, Mutex, MutexGuard, OnceLock};
use camino::{Utf8Path, Utf8PathBuf};
use ort::session::{builder::GraphOptimizationLevel, Session};
use tokenizers::Tokenizer;

pub static IMAGE_SESSION_POOL: LazyLock<SessionPool> = LazyLock::new(|| {
    create_session_pool(1, PoolType::Image)
});

pub static TEXT_SESSION_POOL: LazyLock<SessionPool> = LazyLock::new(|| {
    create_session_pool(1, PoolType::Text)
});

pub static TEXT_TOKENIZER: LazyLock<Tokenizer> = LazyLock::new(|| {
    let base_dir = get_base_resource_dir();
    Tokenizer::from_file(base_dir.join(TOKENIZER_PATH)).expect("Error loading tokenizer from file")
});

pub type SessionPool = Arc<Vec<Mutex<Session>>>;

pub trait SessionPoolExt {
    fn get_session(&'_ self) -> MutexGuard<'_, Session>;
}

impl SessionPoolExt for SessionPool {
    fn get_session(&'_ self) -> MutexGuard<'_, Session> {
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

                let base_dir = get_base_resource_dir();

                let session_result = match pool_type {
                    PoolType::Image => session_builder.commit_from_file(base_dir.join(IMAGE_MODEL_PATH)),
                    PoolType::Text => session_builder.commit_from_file(base_dir.join(TEXT_MODEL_PATH)),
                };

                Mutex::new(session_result.expect("Failed to commit model from memory"))
            })
            .collect()
    )
}

// Private functions and variables

const IMAGE_MODEL_PATH: &str = "siglip2-base-patch16-512/image_embedder.onnx";
const TEXT_MODEL_PATH: &str = "siglip2-base-patch16-512/text_embedder.onnx";
const TOKENIZER_PATH: &str = "siglip2-base-patch16-512/tokenizer.json";

/// Static variable for the base resource (model + tokenizer files) directory
/// Defaults to "models" if not explicitly set
static BASE_RESOURCE_DIRECTORY: OnceLock<Utf8PathBuf> = OnceLock::new();

/// Set the base resource directory. If the base resource directory has already been
/// set or a model has already been loaded, this will be ignored.
pub fn init_model_resource_directory(path: &Utf8Path) {
    BASE_RESOURCE_DIRECTORY.set(path.to_path_buf()).unwrap_or_else(|_| {
        eprintln!("Attempting to change previously resolved base model resource directory, ignoring");
    });
}

/// Get the base resource directory, defaulting to "models"
fn get_base_resource_dir() -> Utf8PathBuf {
    BASE_RESOURCE_DIRECTORY
        .get_or_init(|| Utf8PathBuf::from("models"))
        .clone()
}