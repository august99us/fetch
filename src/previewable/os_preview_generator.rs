use std::fs::DirEntry;

use crate::Preview;

pub fn has_generator_for_type(extension: &str) -> bool {
    let os_generator: Option<Box<dyn OsPreviewGenerator<_>>> = None;

    #[cfg(target_os = "windows")]
    let os_generator = Some(Box::new(WindowsPreviewGenerator));
    
    os_generator.map(|gen| gen.has_generator_for_type(extension)).unwrap_or(false)
}

pub fn generate_preview<R>(entry: &DirEntry) -> Result<Preview<R>, &str> {
    todo!()
}

trait OsPreviewGenerator<R> {
    fn has_generator_for_type(&self, extension: &str) -> bool;
    fn generate_preview(&self, entry: &DirEntry) -> Result<Preview<R>, &str>;
}

struct WindowsPreviewGenerator;
impl<R> OsPreviewGenerator<R> for WindowsPreviewGenerator {
    fn has_generator_for_type(&self, extension: &str) -> bool {
        todo!()
    }

    fn generate_preview(&self, entry: &DirEntry) -> Result<Preview<R>, &str> {
        todo!()
    }
}