use camino::{Utf8Path, Utf8PathBuf};
use log::{error, info};
use ort::execution_providers::*;

use crate::index::embedding::{embeddinggemma, sessions::init_model_resource_directory, siglip2};

/// Initialize dynamic libraries and other dynamic resource paths.
/// Must be called before init_indexing or init_querying
pub fn init_resources(path: Option<&Utf8Path>) -> Result<(), anyhow::Error> {
    let default_path = Utf8PathBuf::default();
    let resource_path = path.unwrap_or(&default_path);

    #[cfg(feature = "pdf")]
    {
        use crate::index::provider::pdf::PDFIUM_LIB_PATH;

        info!("Initializing PDFium...");
        PDFIUM_LIB_PATH.set(resource_path.to_owned())
            .map_err(|_| anyhow::anyhow!("PDFium library path has already been set"))?;
    }

    info!("Initializing ONNX Runtime...");
    init_ort(path)?;

    info!("Initializing base model directory...");
    let base_model_dir = resource_path.join("models");
    init_model_resource_directory(&base_model_dir);

    Ok(())
}

// TODO: implement functionality to init for specific models
pub fn init_indexing(_models: Vec<&str>) {
    // do init for models
    siglip2::init_indexing();
    embeddinggemma::init();
}
pub fn init_querying(_models: Vec<&str>) {
    // do init for models
    siglip2::init_querying();
    embeddinggemma::init();
}

// Private initialization functions

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
fn init_ort(onnx_lib_path: Option<&Utf8Path>) -> Result<(), anyhow::Error> {
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