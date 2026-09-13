use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Debug};

use mtgml_model::PlayerId;

// The module itself is private; these helpers are public only within the
// crate's internal diagnostic implementation boundary.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformanceFailureClass {
    CurrentDecision,
    Response,
    Acceptance,
    Events,
    Delta,
    StateDigest,
    NextDecision,
    Status,
    PlayerProjection,
    RejectedMutation,
}

impl ConformanceFailureClass {
    pub(crate) fn as_token(self) -> &'static str {
        match self {
            Self::CurrentDecision => "current_decision",
            Self::Response => "response",
            Self::Acceptance => "acceptance",
            Self::Events => "events",
            Self::Delta => "delta",
            Self::StateDigest => "state_digest",
            Self::NextDecision => "next_decision",
            Self::Status => "status",
            Self::PlayerProjection => "player_projection",
            Self::RejectedMutation => "rejected_mutation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformanceMismatchKind {
    ValueChanged,
    ExpectedEntryMissing,
    UnexpectedExtraEntry,
    PlayerMissing,
    UnexpectedPlayer,
    PlayerStepDiffered,
    RejectedMutation,
}

impl ConformanceMismatchKind {
    pub(crate) fn as_token(self) -> &'static str {
        match self {
            Self::ValueChanged => "value_changed",
            Self::ExpectedEntryMissing => "expected_entry_missing",
            Self::UnexpectedExtraEntry => "unexpected_extra_entry",
            Self::PlayerMissing => "player_missing",
            Self::UnexpectedPlayer => "unexpected_player",
            Self::PlayerStepDiffered => "player_step_differed",
            Self::RejectedMutation => "rejected_mutation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceDifference {
    pub first_differing_index: usize,
    pub expected_length: usize,
    pub actual_length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceDifference {
    pub surface: ConformanceFailureClass,
    pub semantic_path: String,
    pub mismatch_kind: ConformanceMismatchKind,
    pub expected_summary: String,
    pub actual_summary: String,
    pub sequence: Option<SequenceDifference>,
}

impl ConformanceDifference {
    pub fn signature_marker(&self) -> String {
        format!(
            "MANAFOLD_FAILURE_SIGNATURE v1 surface={} path={} mismatch_kind={}",
            self.surface.as_token(),
            self.semantic_path,
            self.mismatch_kind.as_token(),
        )
    }
}

impl fmt::Display for ConformanceDifference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} expected={} actual={}",
            self.signature_marker(),
            self.expected_summary,
            self.actual_summary,
        )
    }
}

fn value_difference<Expected: Debug, Actual: Debug>(
    surface: ConformanceFailureClass,
    semantic_path: String,
    mismatch_kind: ConformanceMismatchKind,
    expected: &Expected,
    actual: &Actual,
    sequence: Option<SequenceDifference>,
) -> ConformanceDifference {
    ConformanceDifference {
        surface,
        semantic_path,
        mismatch_kind,
        expected_summary: format!("{expected:?}"),
        actual_summary: format!("{actual:?}"),
        sequence,
    }
}

pub(crate) fn compare_value<T: Debug + PartialEq>(
    surface: ConformanceFailureClass,
    path: &str,
    expected: &T,
    actual: &T,
) -> Option<ConformanceDifference> {
    (expected != actual).then(|| {
        value_difference(
            surface,
            path.to_owned(),
            ConformanceMismatchKind::ValueChanged,
            expected,
            actual,
            None,
        )
    })
}

pub(crate) fn compare_sequence<T: Debug + PartialEq>(
    surface: ConformanceFailureClass,
    path: &str,
    expected: &[T],
    actual: &[T],
) -> Option<ConformanceDifference> {
    let sequence = SequenceDifference {
        first_differing_index: 0,
        expected_length: expected.len(),
        actual_length: actual.len(),
    };
    for index in 0..expected.len().min(actual.len()) {
        if expected[index] != actual[index] {
            return Some(value_difference(
                surface,
                format!("{path}[{index}]"),
                ConformanceMismatchKind::ValueChanged,
                &expected[index],
                &actual[index],
                Some(SequenceDifference {
                    first_differing_index: index,
                    ..sequence
                }),
            ));
        }
    }

    if expected.len() > actual.len() {
        let index = actual.len();
        return Some(value_difference(
            surface,
            format!("{path}[{index}]"),
            ConformanceMismatchKind::ExpectedEntryMissing,
            &expected[index],
            &"<missing>",
            Some(SequenceDifference {
                first_differing_index: index,
                ..sequence
            }),
        ));
    }
    if actual.len() > expected.len() {
        let index = expected.len();
        return Some(value_difference(
            surface,
            format!("{path}[{index}]"),
            ConformanceMismatchKind::UnexpectedExtraEntry,
            &"<missing>",
            &actual[index],
            Some(SequenceDifference {
                first_differing_index: index,
                ..sequence
            }),
        ));
    }
    None
}

pub(crate) fn compare_player_map<T: PartialEq>(
    expected: &BTreeMap<PlayerId, T>,
    actual: &BTreeMap<PlayerId, T>,
) -> Option<ConformanceDifference> {
    let players: BTreeSet<PlayerId> = expected.keys().chain(actual.keys()).copied().collect();
    for player in players {
        let path = format!("player_steps[player:{}]", player.0);
        match (expected.get(&player), actual.get(&player)) {
            (Some(_expected), None) => {
                return Some(value_difference(
                    ConformanceFailureClass::PlayerProjection,
                    path,
                    ConformanceMismatchKind::PlayerMissing,
                    &"<present>",
                    &"<missing>",
                    None,
                ));
            }
            (None, Some(_actual)) => {
                return Some(value_difference(
                    ConformanceFailureClass::PlayerProjection,
                    path,
                    ConformanceMismatchKind::UnexpectedPlayer,
                    &"<missing>",
                    &"<present>",
                    None,
                ));
            }
            (Some(expected), Some(actual)) if expected != actual => {
                return Some(value_difference(
                    ConformanceFailureClass::PlayerProjection,
                    path,
                    ConformanceMismatchKind::PlayerStepDiffered,
                    &"<different>",
                    &"<different>",
                    None,
                ));
            }
            (None, None) | (Some(_), Some(_)) => {}
        }
    }
    None
}

pub(crate) fn rejected_mutation_difference(changed: bool) -> Option<ConformanceDifference> {
    changed.then(|| ConformanceDifference {
        surface: ConformanceFailureClass::RejectedMutation,
        semantic_path: "rejected_mutation.next_state".into(),
        mismatch_kind: ConformanceMismatchKind::RejectedMutation,
        expected_summary: "unchanged".into(),
        actual_summary: "changed".into(),
        sequence: None,
    })
}

pub(crate) fn first_difference<I>(differences: I) -> Option<ConformanceDifference>
where
    I: IntoIterator<Item = Option<ConformanceDifference>>,
{
    differences.into_iter().find_map(|difference| difference)
}
