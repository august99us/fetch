use std::{error::Error, fs::File, io::{Bytes, Read}};

pub fn has_generator_for_type(extension: &str) -> bool {
    let os_generator: Option<Box<dyn ImpliesGeneratingPreview>> = None;

    #[cfg(target_os = "windows")]
    let os_generator = Some(Box::new(WindowsPreviewGenerator));
    
    os_generator.map(|gen| gen.has_generator_for_type(extension)).unwrap_or(false)
}

pub fn generate_preview(file: File) -> Result<Bytes<File>, Box<dyn Error>> {
    Ok(file.bytes())
}

trait ImpliesGeneratingPreview {
    fn has_generator_for_type(&self, extension: &str) -> bool;
}
trait GeneratesPreview {
    fn generate_preview(&self, file: File) -> Result<Bytes<File>, Box<dyn Error>>;
}

struct WindowsPreviewGenerator;
impl ImpliesGeneratingPreview for WindowsPreviewGenerator {
    fn has_generator_for_type(&self, extension: &str) -> bool {
        false
    }
}
impl GeneratesPreview for WindowsPreviewGenerator {
    fn generate_preview(&self, file: File) -> Result<Bytes<File>, Box<dyn Error>> {
        Ok(file.bytes())
    }
}