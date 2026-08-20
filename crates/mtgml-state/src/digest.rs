use mtgml_model::StateRevision;
use serde_json::{Map as JsonMap, Value as JsonValue};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum StateDigestError {
    #[error("canonical full-state digest serialization failed")]
    Serialization,
}

#[derive(serde::Serialize)]
pub(crate) struct FullStateDigestInputV2<'a> {
    pub schema_version: &'static str,
    pub domain: &'static str,
    pub revision: StateRevision,
    pub core: &'a crate::core::CoreRulesState,
    pub zones: crate::zones::CanonicalZoneStateV1<'a>,
    pub allocators: &'a crate::identity::IdentityAllocatorState,
    pub execution: &'a crate::execution::ExecutionState,
    pub random: &'a mtgml_random::RandomStateV1,
    pub knowledge: &'a crate::knowledge::KnowledgeState,
    pub perspective_identities: &'a crate::identity::PerspectiveIdentityState,
    pub format: &'a crate::format::FormatState,
}

pub(crate) fn canonicalize_json(value: JsonValue) -> JsonValue {
    match value {
        JsonValue::Array(items) => {
            JsonValue::Array(items.into_iter().map(canonicalize_json).collect())
        }
        JsonValue::Object(object) => {
            let mut entries: Vec<_> = object.into_iter().collect();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            let mut sorted = JsonMap::new();
            for (key, value) in entries {
                sorted.insert(key, canonicalize_json(value));
            }
            JsonValue::Object(sorted)
        }
        scalar => scalar,
    }
}
