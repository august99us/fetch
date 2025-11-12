use std::{cmp::Ordering, collections::HashMap, future::Future};

use camino::{Utf8Path, Utf8PathBuf};
use chrono::Utc;
use log::{debug, warn};

use crate::{files::{ChunkingIndexProviderConcurrent, pagination::{AggregateFileScore, QueryCursor, TTL_ATTR}}, store::{ClearByFilter, Filter, FilterRelation, FilterValue, KeyedSequencedStore}};

use super::FileQueryer;

/// Describes an object that understands how to perform queries against indexed files.
/// 
/// This trait provides methods for finding files that are similar to a given
/// text description. Both methods support cursor-based pagination to handle large
/// and living result lists.
/// 
/// The result types in this API return three things:
/// 1. A list length - this represents the current length of the living list that the
///    client is expected to maintain.
/// 2. A list of changed elements - the elements that have changed from the current client-
///    side picture of the results list, as well as their new values.
/// 3. A cursor id - this will need to be passed in as a part of future queries to query
///    further chunks and continue aggregating in the same cursor
///    
/// An example of how this will work:
///     Client-side currently has:
///     {
///         query: "dog"
///         results_len: 3,
///         results: [(a.txt, 84), (b.jpg, 82), (c.psd, 40)],
///         cursor_id: "id1",
///     }
///     Client-side will pass in:
///     {
///         query: "dog"
///         cursor_id: "id1",
///     }
///     API Response will look like:
///     {
///         results_len: 4,
///         changed_results: [(rank: 2, old_rank: 1, a.txt, 84),
///                           (rank: 1, old_rank: 2, b.jpg, 86),
///                           (rank: 4, d.md, 22)],
///         cursor_id: "id1",
///     }
/// 
/// These results indicate that one more result has been found than before, the results a
/// and b have swapped positions, the b.jpg score has increased from 82 -> 86, and the 4th
/// result that has been found, the new one, is d.md with a score of 22. The new results
/// list should now look like:
/// 
/// [(b.jpg, 86), (a.txt, 84), (c.psd, 40), (d.md, 22)]
/// 
/// The changed positions will guaranteed always resolve without duplicates or removals.
/// Ranks are always 1-indexed.
/// 
/// Finally, if there is no cursor id returned in the response, then that means the client
/// has reached the end of the list and should not query the cursor any further.
pub trait QueryFiles {
    /// Query for files matching description provided, parsing through a default number of chunks
    /// (currently 100) and aggregating them into the cursor. This API will only return new results
    /// not seen before in previous queries with the same cursor id. If a cursor id is not provided,
    /// then the query is a new query and a cursor id will be created and returned as a part of the
    /// result.
    /// 
    /// # Arguments
    /// * `query_terms` - The text description to query for
    /// * `cursor_id` - Optional cursor-id. If None, then it will be assumed that this is a new query
    /// 
    /// # Returns
    /// Returns the new list length and the change in results for the aggregating cursor.
    /// If no cursor id is returned in the results, then that means the end of the list of chunks has
    /// been reached.
    async fn query(&self, query_terms: &str, cursor_id: Option<&str>) -> Result<FileQueryingResult, FileQueryingError>;
    
    /// Query for files matching description provided, parsing through a given number of chunks
    /// per query and aggregating them into the cursor. This API will only return new results not
    /// returned before in previous queries with the same cursor id.
    /// 
    /// # Arguments
    /// * `query_terms` - The text description to search for
    /// * `num_results` - Number of results to return per page
    /// * `page` - Page number (1-based). Page 1 returns results 1-num_results, page 2 returns
    ///   results (num_results+1)-(2*num_results), etc.
    /// 
    /// # Returns
    /// Returns the new list length and the change in results for the aggregating cursor.
    /// If no cursor id is returned in the results, then that means the end of the list of chunks has
    /// been reached.
    async fn query_n(&self, query_terms: &str, num_chunks: u32, cursor_id: Option<&str>) -> Result<FileQueryingResult, FileQueryingError>;
}

impl<C> QueryFiles for FileQueryer<C>
where
    C: KeyedSequencedStore<String, QueryCursor> +
        ClearByFilter<QueryCursor> +
        Send + Sync
{
    // Query 20 results by default, starting from page 1 if no page specified
    fn query(&self, query_terms: &str, cursor_id: Option<&str>) -> impl Future<Output = Result<FileQueryingResult, FileQueryingError>> {
        self.query_n(query_terms, 20, cursor_id)
    }

    async fn query_n(&self, query_terms: &str, num_chunks: u32, cursor_id: Option<&str>) -> Result<FileQueryingResult, FileQueryingError> {
        debug!("FileQueryer: Querying indexes with parameters: {}, num_chunks: {}, cursor_id: {:?}",
            query_terms, num_chunks, cursor_id);
        let mut cursor;
        if let Some(cur_id) = cursor_id {
            debug!("FileQueryer: Retrieving cursor with id: {}", cur_id);
            let o_cursor = self.cursor_store.get(cur_id.to_string()).await
                .map_err(|e| FileQueryingError {
                    query: query_terms.to_owned(),
                    r#type: FileQueryingErrorType::CursorStore { source: e.into() },
                })?;
            if o_cursor.is_none() {
                return Err(FileQueryingError {
                    query: query_terms.to_owned(),
                    r#type: FileQueryingErrorType::CursorNotFound,
                });
            }
            cursor = o_cursor.unwrap();
        } else {
            cursor = QueryCursor::fresh();
            debug!("Initialized new cursor with id: {}", cursor.id);
        }

        // clear ttl (TODO: Build a database interface that supports automatically clearing ttl)
        debug!("FileQueryer: Clearing expired cursors from cursor store using clear_filter and ttl field");
        self.cursor_store.clear_filter(&[Filter {
            attribute: TTL_ATTR,
            filter: FilterValue::DateTime(&Utc::now()),
            relation: FilterRelation::Lt
        }]).await
            .map_err(|e| FileQueryingError {
                query: query_terms.to_owned(),
                r#type: FileQueryingErrorType::CursorStore { source: e.into() },
            })?;

        let old_hash = cursor.aggregate_scores.clone();
        let rankmap = produce_rankmap(&old_hash);
        let original_len = cursor.aggregate_scores.len() as u32;

        debug!("FileQueryer: Performing provider queries for query: {}", query_terms);
        let query_copy = query_terms.to_owned();
        let results = self.index_providers.distribute_calls(async move |p| {
            p.query_n(&query_copy, num_chunks, cursor.curr_offset).await
        }).await.map_err(|e| FileQueryingError {
            query: query_terms.to_owned(),
            r#type: FileQueryingErrorType::Other {
                msg: "Join error occurred while querying indexes",
                source: e,
            },
        })?;
        let mut has_results = false;
        let mut provider_error_map = HashMap::new();
        for res in results {
            match res {
                Ok(vec) => {
                    if !vec.is_empty() {
                        has_results = true;

                        for cqr in vec {
                            cursor.aggregate_chunk(&cqr.chunkfile().original_file, cqr.score());
                        }
                    }
                },
                Err(e) => {
                    let provider_name = e.provider_name.clone();
                    provider_error_map.insert(provider_name, e);
                }
            }
        }
        if !provider_error_map.is_empty() {
            if provider_error_map.len() == self.index_providers.len() {
                debug!("FileQueryer: All index providers returned errors for query: {}", query_terms);
                return Err(FileQueryingError {
                    query: query_terms.to_owned(),
                    r#type: FileQueryingErrorType::IndexProviders { provider_errors: provider_error_map },
                });
            } else {
                warn!("FileQueryer: Some index providers returned errors for query: {}. Ignoring \
                    to allow other providers to return results", query_terms);
            }
        }
        
        if !has_results {
            debug!("FileQueryer: Found no more results, returning empty result (same length, empty changed, empty cursor)");
            return Ok(FileQueryingResult {
                results_len: original_len,
                changed_results: vec![],
                cursor_id: None,
            })
        }

        debug!("FileQueryer: Calculating changed results from new and old aggregated cursor data");
        // borrow the cursor aggregate score hashmap's values to calculate result
        let mut new_list: Vec<_> = cursor.aggregate_scores.iter().collect();
        new_list.sort_by(cmp_score_entries_desc);

        // calculate changed ranks and scores and save copied versions in changed_vec
        let mut changed_vec = vec![];
        for (rank, entry) in new_list.iter().enumerate() {
            let rank = (rank + 1) as u32;
            let res_path = entry.0.as_path();
            let score = entry.1.chunk_multiplier_score();
            let old_rank_opt = rankmap.get(res_path);
            if let Some(old_rank) = old_rank_opt {
                let old_score = old_hash.get(res_path)
                    .expect("result exists in old rank map but not in hash map, should not happen")
                    .chunk_multiplier_score();

                if *old_rank == rank && old_score == score {
                    continue;
                }
            }

            changed_vec.push(QueryResult {
                old_rank: old_rank_opt.copied(),
                rank,
                path: entry.0.clone(),
                score,
            })
        }
        // drop immutable borrow on cursor aggregate score hashmap
        drop(new_list);

        // pre-prepare other cursor values that need to be returned to client
        let new_list_len = cursor.aggregate_scores.len() as u32;
        let new_cursor_id = cursor.id.clone();

        debug!("FileQueryer: Updating and saving cursor with id: {}", cursor.id);
        // update and save cursor, consuming it
        cursor.curr_offset += num_chunks;
        cursor.touch_ttl();
        self.cursor_store.put(vec![cursor]).await
            .map_err(|e| FileQueryingError {
                query: query_terms.to_owned(),
                r#type: FileQueryingErrorType::CursorStore { source: e.into() },
            })?;

        Ok(FileQueryingResult {
            results_len: new_list_len,
            changed_results: changed_vec,
            cursor_id: Some(new_cursor_id),
        })
    }
}

pub use result::*;
pub use error::*;

// private methods and modules

fn produce_rankmap(original: &HashMap<Utf8PathBuf, AggregateFileScore>) -> HashMap<&Utf8Path, u32> {
    let mut original_list: Vec<_> = original.iter().collect();
    original_list.sort_by(cmp_score_entries_desc);

    let mut rankmap = HashMap::new();
    for ordered_elem in original_list.iter().enumerate() {
        rankmap.insert(ordered_elem.1.0.as_path(), (ordered_elem.0 + 1) as u32);
    }
    rankmap
}

fn cmp_score_entries_desc(
    l: &(impl AsRef<Utf8Path>, impl AsRef<AggregateFileScore>),
    r: &(impl AsRef<Utf8Path>, impl AsRef<AggregateFileScore>)
) -> Ordering {
    // r.compare(l) to reverse ordering for descending
    let cmp = (r.1.as_ref().chunk_multiplier_score())
        .total_cmp(&(l.1.as_ref().chunk_multiplier_score()));

    if cmp.is_eq() {
        // If scores are equal, then just return the ordering of the filenames instead
        r.0.as_ref().to_string().cmp(&l.0.as_ref().to_string())
    } else {
        cmp
    }
}

mod result;
mod error;