use mtgml_model::VisibleSequence;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeHistoryChannel {
    Public,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgePoint {
    pub channel: KnowledgeHistoryChannel,
    pub sequence: VisibleSequence,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeAcquisitionCause {
    PublicEvent,
    PrivateLook,
    ExplicitReveal,
    OwnPrivateIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum KnowledgeAcquisitionReason {
    InitialConfiguration,
    Observed {
        channel: KnowledgeHistoryChannel,
        sequence: VisibleSequence,
        cause: KnowledgeAcquisitionCause,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeInvalidationReason {
    Shuffle,
    Randomization,
    HiddenTransition,
    ExplicitForget,
}
