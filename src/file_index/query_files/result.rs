use std::ops::{Deref, DerefMut};

use camino::Utf8PathBuf;

use crate::vector_store::QueryKeyResult;

pub struct FileQueryingResult {
    results: Vec<QueryResult>,
}
impl Deref for FileQueryingResult {
    type Target = Vec<QueryResult>;
    
    fn deref(&self) -> &Self::Target {
        &self.results
    }
}
impl DerefMut for FileQueryingResult {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.results
    }
}
impl IntoIterator for FileQueryingResult {
    type Item = QueryResult;
    type IntoIter = std::vec::IntoIter<QueryResult>;

    fn into_iter(self) -> Self::IntoIter {
        self.results.into_iter()
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