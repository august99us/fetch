use std::ops::Deref;

use camino::Utf8PathBuf;

use crate::vector_store::QueryKeyResult;

pub struct FileQueryingResult {
    results: Vec<QueryResult>,
}
impl<'a> Deref for FileQueryingResult {
    type Target = Vec<QueryResult>;
    
    fn deref(&self) -> &Self::Target {
        &self.results
    }
}
impl From<Vec<QueryKeyResult>> for FileQueryingResult {
    fn from(value: Vec<QueryKeyResult>) -> Self {
        FileQueryingResult { results: value.into_iter().map(QueryResult::from).collect() }
    }
}

pub struct QueryResult {
    pub path: Utf8PathBuf,
    pub similarity: f32,
}
impl From<QueryKeyResult> for QueryResult {
    fn from(value: QueryKeyResult) -> QueryResult {
        QueryResult {
            path: Utf8PathBuf::from(value.key),
            similarity: value.distance, //1.0/(value.distance + 1.0), // temporarily using distance as metric
        }
    }
}