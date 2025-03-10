use std::{fs::File, io::Read, path::Path, time::SystemTime};

use crate::Preview;

pub fn has_generator_for_type(extension: &str) -> bool {
    match extension {
        "jpg" => true,
        _ => false,
    }
}

pub fn generate_preview(entry: &Path) -> Result<Preview<File>, String> {
    let file = File::open(entry).map_err(|e| e.to_string())?;

    Ok(Preview {
        content: file.bytes(),
        // todo: error handling
        path: entry.to_str().unwrap().to_owned(),
        timestamp: SystemTime::now(),
        r#type: crate::PreviewType::Image,
    })
}