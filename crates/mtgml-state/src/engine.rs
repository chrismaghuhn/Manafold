use mtgml_model::{FullStateDigestV2, StateRevision};
use mtgml_random::RandomStateV1;
use serde::{Deserialize, Serialize};

use crate::core::CoreRulesState;
use crate::digest::{canonicalize_json, FullStateDigestInputV2, StateDigestError};
use crate::execution::ExecutionState;
use crate::format::FormatState;
use crate::identity::IdentityAllocatorState;
use crate::knowledge::KnowledgeState;
use crate::zones::{CanonicalOrderedZoneEntryV1, CanonicalZoneStateV1, ZoneState};

pub const FULL_STATE_DIGEST_INPUT_SCHEMA: &str = "full-state-digest-input.v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineState {
    pub revision: StateRevision,
    pub core: CoreRulesState,
    pub zones: ZoneState,
    pub allocators: IdentityAllocatorState,
    pub execution: ExecutionState,
    pub random: RandomStateV1,
    pub knowledge: KnowledgeState,
    pub perspective_identities: crate::identity::PerspectiveIdentityState,
    pub format: FormatState,
}

impl EngineState {
    pub fn canonical_digest_bytes(&self) -> Result<Vec<u8>, StateDigestError> {
        let ordered_zones = self
            .zones
            .ordered_zones
            .iter()
            .map(|(key, objects)| CanonicalOrderedZoneEntryV1 {
                key,
                objects: objects.as_slice(),
            })
            .collect();
        let input = FullStateDigestInputV2 {
            schema_version: FULL_STATE_DIGEST_INPUT_SCHEMA,
            domain: FullStateDigestV2::DOMAIN,
            revision: self.revision,
            core: &self.core,
            zones: CanonicalZoneStateV1 {
                objects: &self.zones.objects,
                locations: &self.zones.locations,
                ordered_zones,
                stack_records: &self.zones.stack_records,
                stack_order: self.zones.stack_order.as_slice(),
            },
            allocators: &self.allocators,
            execution: &self.execution,
            random: &self.random,
            knowledge: &self.knowledge,
            perspective_identities: &self.perspective_identities,
            format: &self.format,
        };
        let value = serde_json::to_value(&input).map_err(|_| StateDigestError::Serialization)?;
        serde_json::to_vec(&canonicalize_json(value)).map_err(|_| StateDigestError::Serialization)
    }

    pub fn digest(&self) -> Result<FullStateDigestV2, StateDigestError> {
        self.canonical_digest_bytes()
            .map(|bytes| FullStateDigestV2::from_canonical_bytes(&bytes))
    }

    pub fn parts(&self) -> EngineStateParts {
        EngineStateParts {
            revision: self.revision,
            core: self.core.clone(),
            zones: self.zones.clone(),
            allocators: self.allocators.clone(),
            execution: self.execution.clone(),
            random: self.random.clone(),
            knowledge: self.knowledge.clone(),
            perspective_identities: self.perspective_identities.clone(),
            format: self.format.clone(),
        }
    }

    pub fn consume_raw_u64(
        &mut self,
        key: &mtgml_random::RandomStreamKeyV1,
    ) -> Result<u64, mtgml_random::RandomValidationError> {
        let root = self.random.root_seed;
        let cursor = self.random.lookup_stream(key)?;
        let (word, next) = mtgml_random::hmac_counter::next_raw_u64(&root, key, &cursor)?;
        self.random.set_cursor(key, next)?;
        Ok(word)
    }

    pub fn uniform_below_u64(
        &mut self,
        key: &mtgml_random::RandomStreamKeyV1,
        n: u64,
    ) -> Result<(u64, u64), mtgml_random::RandomValidationError> {
        let current = self.random.lookup_stream(key)?;
        let (value, consumed, next) =
            mtgml_random::sampling::uniform_below_u64(&self.random.root_seed, key, &current, n)?;
        self.random.set_cursor(key, next)?;
        Ok((value, consumed))
    }

    pub fn shuffle<T: Clone>(
        &mut self,
        values: &mut [T],
        key: &mtgml_random::RandomStreamKeyV1,
    ) -> Result<u64, mtgml_random::RandomValidationError> {
        let cursor = self.random.lookup_stream(key)?;
        let (consumed, next) =
            mtgml_random::sampling::shuffle(values, &self.random.root_seed, key, &cursor)?;
        self.random.set_cursor(key, next)?;
        Ok(consumed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineStateParts {
    pub revision: StateRevision,
    pub core: CoreRulesState,
    pub zones: ZoneState,
    pub allocators: IdentityAllocatorState,
    pub execution: ExecutionState,
    pub random: RandomStateV1,
    pub knowledge: KnowledgeState,
    pub perspective_identities: crate::identity::PerspectiveIdentityState,
    pub format: FormatState,
}

impl From<EngineStateParts> for EngineState {
    fn from(parts: EngineStateParts) -> Self {
        Self {
            revision: parts.revision,
            core: parts.core,
            zones: parts.zones,
            allocators: parts.allocators,
            execution: parts.execution,
            random: parts.random,
            knowledge: parts.knowledge,
            perspective_identities: parts.perspective_identities,
            format: parts.format,
        }
    }
}
