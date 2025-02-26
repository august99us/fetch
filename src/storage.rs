use crate::previewable::Preview;

pub trait IndexPreview {
    fn index<R>(preview: Preview<R>) -> Result<(), &'static str>;
}

pub trait QuerySimilarFiles {
    fn query(file_description: String) -> String;
}

pub mod lancedb_store;