use std::io::Read;

use crate::Preview;

pub type Embedding = [f32; 1000];

pub trait Embeddable {
    fn calculate_embedding(&self) -> Result<Embedding, &'static str>;
}

impl<R: Read> Embeddable for Preview<R> {
    fn calculate_embedding(&self) -> Result<Embedding, &'static str> {
        todo!()
    }
}

impl Embeddable for &str {
    fn calculate_embedding(&self) -> Result<Embedding, &'static str> {
        todo!()
    }
}