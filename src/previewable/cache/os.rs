use std::fs::File;

use camino::{Utf8Path, Utf8PathBuf};

pub fn has_generator_for_type(extension: &str) -> bool {
    let os_generator: Option<Box<dyn ImpliesGeneratingPreview>> = None;

    #[cfg(target_os = "windows")]
    let os_generator = Some(Box::new(WindowsPreviewGenerator));
    
    os_generator.map(|gen| gen.has_generator_for_type(extension)).unwrap_or(false)
}

pub fn generate_preview(path: &Utf8Path) -> Result<Option<Utf8PathBuf>, anyhow::Error> {
    Ok(Some(Utf8PathBuf::new()))
}

trait ImpliesGeneratingPreview {
    fn has_generator_for_type(&self, extension: &str) -> bool;
}
trait GeneratesPreview {
    fn generate_preview(&self, file: File) -> Result<Utf8PathBuf, anyhow::Error>;
}

struct WindowsPreviewGenerator;
impl ImpliesGeneratingPreview for WindowsPreviewGenerator {
    fn has_generator_for_type(&self, extension: &str) -> bool {
        false
    }
}
impl GeneratesPreview for WindowsPreviewGenerator {
    fn generate_preview(&self, file: File) -> Result<Utf8PathBuf, anyhow::Error> {
        Ok(Utf8PathBuf::new())
    }
}