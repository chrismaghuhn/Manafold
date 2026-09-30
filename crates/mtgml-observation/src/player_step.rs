//! Ownership: player-step DTOs (V1 and V2), submission/service codes, and
//! their existing validation / `code()` logic.

use serde::{Deserialize, Serialize};

/// Versioned closed codes for typed player submission rejections
/// (ERROR_MODEL, layer B). Wire representation: snake_case strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerSubmissionCodeV1 {
    StaleDecision,
    UnavailableDecision,
    InvalidAnswer,
    InvalidCandidate,
    DuplicateAssignment,
    InvalidCardinality,
    InvalidNumber,
    InvalidOrder,
    EpisodeClosed,
}

/// Versioned closed service-failure code (ERROR_MODEL, layer C).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerServiceErrorCodeV1 {
    ServiceUnavailable,
}

/// Versioned submission outcome carried by `PlayerStepV2`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlayerStepSubmissionV1 {
    Accepted,
    Rejected { code: PlayerSubmissionCodeV1 },
}

impl PlayerServiceErrorCodeV1 {
    /// Single versioned authority for the public service-failure string.
    pub fn code(self) -> &'static str {
        match self {
            Self::ServiceUnavailable => "service_unavailable",
        }
    }
}
