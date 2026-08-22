use mtgml_model::VisibleSequence;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeHistoryChannel {
    Public,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeAcquisitionCause {
    PublicEvent,
    PrivateLook,
    ExplicitReveal,
    OwnPrivateIdentity,
}

/// Complete typed provenance for one retained semantic fact.
///
/// The authoritative state owns the exact observed cause; downstream layers
/// (digest, projection) consume this value and never infer a cause from the
/// channel. `InitialConfiguration` carries no visible sequence; an observed
/// fact binds channel, sequence, and cause explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum KnowledgeAcquisitionReason {
    InitialConfiguration,
    Observed {
        channel: KnowledgeHistoryChannel,
        sequence: VisibleSequence,
        cause: KnowledgeAcquisitionCause,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeInvalidationReason {
    Shuffle,
    Randomization,
    HiddenTransition,
    ExplicitForget,
}

impl KnowledgeAcquisitionReason {
    /// The observed visible sequence of this provenance, if any.
    pub fn observed_sequence(&self) -> Option<VisibleSequence> {
        match self {
            Self::InitialConfiguration => None,
            Self::Observed { sequence, .. } => Some(*sequence),
        }
    }

    /// Whether the declared channel and cause are an accepted combination:
    /// public facts are `public_event`/`explicit_reveal`; private facts are
    /// `private_look`/`own_private_identity`.
    pub fn has_accepted_channel_cause(&self) -> bool {
        match self {
            Self::InitialConfiguration => true,
            Self::Observed {
                channel,
                cause: KnowledgeAcquisitionCause::PublicEvent,
                ..
            }
            | Self::Observed {
                channel,
                cause: KnowledgeAcquisitionCause::ExplicitReveal,
                ..
            } => *channel == KnowledgeHistoryChannel::Public,
            Self::Observed {
                channel,
                cause: KnowledgeAcquisitionCause::PrivateLook,
                ..
            }
            | Self::Observed {
                channel,
                cause: KnowledgeAcquisitionCause::OwnPrivateIdentity,
                ..
            } => *channel == KnowledgeHistoryChannel::Private,
        }
    }

    /// Whether this provenance is valid inside a perspective whose next
    /// unused visible sequence is `next_visible_sequence`.
    pub fn is_within_visible_sequence(&self, next_visible_sequence: VisibleSequence) -> bool {
        match self.observed_sequence() {
            None => true,
            Some(sequence) => sequence.0 < next_visible_sequence.0,
        }
    }
}
