use camino::Utf8Path;

// Perhaps this needs to be a struct so path can be a common variable amongst all variants?
pub enum FileIndexingResultType {
    Indexed,
    Skipped { reason: String },
    Cleared,
}
pub struct FileIndexingResult<'a> {
    pub path: &'a Utf8Path,
    pub r#type: FileIndexingResultType,
}
