use std::error::Error;

use ort::execution_providers::*;

/// Initialize ONNX Runtime
pub fn init_ort() -> Result<(), Box<dyn Error>> {
    let mut execution_providers = vec![];
    
    #[cfg(feature = "qnn")]
    execution_providers.push(QNNExecutionProvider::default()
        .with_backend_path("QnnHtp.dll")
        .build()
        .error_on_failure());
    #[cfg(feature = "cuda")]
    execution_providers.push(CUDAExecutionProvider::default().build().error_on_failure());
    execution_providers.push(CPUExecutionProvider::default().build());

    match ort::init().with_execution_providers(execution_providers).commit() {
        Ok(_) => {
            println!("ONNX Runtime initialized successfully");
            Ok(())
        },
        Err(e) => {
            eprintln!("Failed to initialize ONNX Runtime: {}", e);
            Err(e.into())
        }
    }
}