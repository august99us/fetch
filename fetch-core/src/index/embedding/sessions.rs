use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use camino::{Utf8Path, Utf8PathBuf};
use log::warn;
use ort::session::{builder::GraphOptimizationLevel, Session};
use tokenizers::Tokenizer;

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

pub fn create_session_pool(pool_size: u32, model_path: &Utf8Path) -> SessionPool {
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

                let session_result = session_builder.commit_from_file(base_dir.join(model_path));

                Mutex::new(session_result.expect("Failed to commit model from memory"))
            })
            .collect()
    )
}

pub fn create_tokenizer(tokenizer_path: &Utf8Path) -> Tokenizer {
    let base_dir = get_base_resource_dir();
    Tokenizer::from_file(base_dir.join(tokenizer_path)).expect("Error loading tokenizer from file")
}

/// Static variable for the base resource (model + tokenizer files) directory
/// Defaults to "models" if not explicitly set
static BASE_RESOURCE_DIRECTORY: OnceLock<Utf8PathBuf> = OnceLock::new();

/// Set the base resource directory. If the base resource directory has already been
/// set or a model has already been loaded, this will be ignored.
pub(crate) fn init_model_resource_directory(path: &Utf8Path) {
    BASE_RESOURCE_DIRECTORY.set(path.to_path_buf()).unwrap_or_else(|_| {
        warn!("Attempting to change previously resolved base model resource directory, ignoring");
    });
}

/// Get the base resource directory, defaulting to "models"
pub(crate) fn get_base_resource_dir() -> Utf8PathBuf {
    BASE_RESOURCE_DIRECTORY
        .get_or_init(|| Utf8PathBuf::from("models"))
        .clone()
}