use std::error::Error;
use camino::Utf8Path;
use log::{error, info};
use ort::execution_providers::*;

/// Initialize ONNX Runtime with optional library path
///
/// If `onnx_lib_path` is provided, ONNX Runtime will load its dynamic library
/// from that directory. This is useful for bundled applications where libraries are in
/// a specific resource directory.
///
/// The function will look for the platform-specific library name:
/// - Windows: onnxruntime.dll
/// - Linux: libonnxruntime.so
/// - macOS: libonnxruntime.dylib
pub fn init_ort(onnx_lib_path: Option<&Utf8Path>) -> Result<(), Box<dyn Error>> {
    let mut execution_providers = vec![];

    #[cfg(feature = "qnn")]
    {
        let qnn_backend = if let Some(lib_dir) = onnx_lib_path {
            lib_dir.join("QnnHtp.dll").to_string()
        } else {
            "QnnHtp.dll".to_string()
        };

        execution_providers.push(QNNExecutionProvider::default()
            .with_backend_path(qnn_backend)
            .build()
            .error_on_failure());
    }

    #[cfg(feature = "cuda")]
    execution_providers.push(CUDAExecutionProvider::default().build().error_on_failure());
    execution_providers.push(CPUExecutionProvider::default().build());

    let result = if let Some(lib_dir) = onnx_lib_path {
        // Construct the full path to the ONNX Runtime library
        #[cfg(windows)]
        let lib_name = "onnxruntime.dll";

        #[cfg(target_os = "macos")]
        let lib_name = "libonnxruntime.dylib";

        #[cfg(all(not(windows), not(target_os = "macos")))]
        let lib_name = "libonnxruntime.so";

        let lib_path = lib_dir.join(lib_name);

        // Use init_from to load from the specific path
        ort::init_from(&lib_path)
            .with_execution_providers(execution_providers)
            .commit()
    } else {
        // Use default initialization (searches standard paths)
        ort::init()
            .with_execution_providers(execution_providers)
            .commit()
    };

    match result {
        Ok(_) => {
            info!("ONNX Runtime initialized successfully");
            Ok(())
        },
        Err(e) => {
            error!("Failed to initialize ONNX Runtime: {}", e);
            Err(e.into())
        }
    }
}

// Re-export from session_pool
pub use crate::index::embedding::sessions::init_model_resource_directory;
use crate::index::embedding::siglip2_image_embedder;

// TODO: implement functionality to init for specific models
pub fn init_indexing(base_model_dir: Option<&Utf8Path>, _models: Vec<&str>) {
    if let Some(dir) = base_model_dir {
        init_model_resource_directory(dir);
    }

    // do init for models
    siglip2_image_embedder::init_indexing();
}
pub fn init_querying(base_model_dir: Option<&Utf8Path>, _models: Vec<&str>) {
    if let Some(dir) = base_model_dir {
        init_model_resource_directory(dir);
    }

    // do init for models
    siglip2_image_embedder::init_querying();
}