use std::fs::DirEntry;

use crate::preview::Preview;

pub fn has_generator_for_type(extension: &str) -> bool {
    let os_generator: Option<Box<dyn OsPreviewGenerator>> = None;

    #[cfg(target_os = "windows")]
    let os_generator = Some(Box::new(WindowsPreviewGenerator));
    
    os_generator.map(|gen| gen.has_generator_for_type(extension)).unwrap_or(false)
}

pub fn generate_preview(entry: &DirEntry) -> Option<Preview> {
    todo!()
}

trait OsPreviewGenerator {
    fn has_generator_for_type(&self, extension: &str) -> bool;
    fn generate_preview(&self, entry: &DirEntry) -> Option<Preview>;
}

struct WindowsPreviewGenerator;
impl OsPreviewGenerator for WindowsPreviewGenerator {
    fn has_generator_for_type(&self, extension: &str) -> bool {
        todo!()
    }

    fn generate_preview(&self, entry: &DirEntry) -> Option<Preview> {
        todo!()
    }
}