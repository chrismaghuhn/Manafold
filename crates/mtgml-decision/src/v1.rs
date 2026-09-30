use crate::common::CandidateIntent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionCandidate {
    pub candidate_id: String,
    pub semantic_key: String,
    pub intent: CandidateIntent,
}
