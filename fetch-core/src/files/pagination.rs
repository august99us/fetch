use std::collections::HashMap;

use camino::{Utf8Path, Utf8PathBuf};
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateFileScore {
    pub max_score: f32,
    pub num_chunks: u32,
}

impl AggregateFileScore {
    pub fn aggregate_score(&mut self, score: f32) {
        self.max_score = self.max_score.max(score);
        self.num_chunks += 1;
    }

    pub fn chunk_multiplier_score(&self) -> f32 {
        self.max_score * 1.0 + (0.01 * self.num_chunks as f32)
    }
}

impl AsRef<AggregateFileScore> for AggregateFileScore {
    fn as_ref(&self) -> &AggregateFileScore {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCursor {
    pub id: String,
    pub aggregate_scores: HashMap<Utf8PathBuf, AggregateFileScore>,
    pub curr_offset: u32,
    pub ttl: DateTime<Utc>,
}

impl QueryCursor {
    pub fn fresh() -> Self {
        let mut cursor = QueryCursor {
            id: Uuid::new_v4().to_string(),
            aggregate_scores: HashMap::new(),
            curr_offset: 0,
            ttl: Utc::now(),
        };
        cursor.touch_ttl();
        cursor
    }

    pub fn touch_ttl(&mut self) -> &mut Self {
        self.ttl = Utc::now().checked_add_signed(TimeDelta::minutes(5))
            .expect("Added 5 minutes, resulting date out of range");
        self
    }

    pub fn aggregate_chunk(&mut self, file_ref: &Utf8Path, score: f32) -> &mut Self {
        match self.aggregate_scores.get_mut(file_ref) {
            Some(ags) => {
                ags.aggregate_score(score);
            },
            None => {
                self.aggregate_scores.insert(file_ref.to_owned(),
                    AggregateFileScore { max_score: score, num_chunks: 1 });
            },
        }
        self
    }
}

pub use integrations::*;

pub mod integrations;