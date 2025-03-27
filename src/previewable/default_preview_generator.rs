use std::{error::Error, fs::File, io::{Bytes, Read}};

pub fn has_generator_for_type(extension: &str) -> bool {
    match extension {
        "jpg" => true,
        _ => false,
    }
}

pub fn generate_preview(file: File) -> Result<Bytes<File>, Box<dyn Error>> {
    Ok(file.bytes())
}