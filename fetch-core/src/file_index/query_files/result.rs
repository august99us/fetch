use camino::Utf8PathBuf;

pub struct FileQueryingResult {
    pub results_len: u32,
    pub changed_results: Vec<QueryResult>,
    pub cursor_id: Option<String>,
}

pub struct QueryResult {
    pub old_rank: Option<u32>,
    pub rank: u32,
    pub path: Utf8PathBuf,
    pub score: f32,
}