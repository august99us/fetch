use std::path::Path;

use crate::Preview;

pub fn has_generator_for_type(extension: &str) -> bool {
    let os_generator: Option<Box<dyn ImpliesGeneratingPreview>> = None;

    #[cfg(target_os = "windows")]
    let os_generator = Some(Box::new(WindowsPreviewGenerator));
    
    os_generator.map(|gen| gen.has_generator_for_type(extension)).unwrap_or(false)
}

pub fn generate_preview<R>(entry: &Path) -> Result<Preview<R>, String> {
    todo!()
}

trait ImpliesGeneratingPreview {
    fn has_generator_for_type(&self, extension: &str) -> bool;
}
trait GeneratesPreview<R> {
    fn generate_preview(&self, entry: &Path) -> Result<Preview<R>, String>;
}

struct WindowsPreviewGenerator;
impl ImpliesGeneratingPreview for WindowsPreviewGenerator {
    fn has_generator_for_type(&self, extension: &str) -> bool {
        false
    }
}
impl<R> GeneratesPreview<R> for WindowsPreviewGenerator {
    fn generate_preview(&self, entry: &Path) -> Result<Preview<R>, String> {
        Err("not implemented".to_owned())
    }
}