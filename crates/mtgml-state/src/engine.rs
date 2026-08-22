use mtgml_model::{FullStateDigestV3, StateRevision};
use mtgml_random::RandomStateV1;
use serde::{Deserialize, Serialize};

use crate::core::CoreRulesState;
use crate::digest::StateDigestError;
use crate::execution::ExecutionState;
use crate::format::FormatState;
use crate::identity::IdentityAllocatorState;
use crate::m2_shape::{KnowledgeStateV2, PerspectiveIdentityStateV2};
use crate::zones::ZoneState;

pub const FULL_STATE_DIGEST_INPUT_SCHEMA: &str = "full-state-digest-input.v3";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineState {
    pub revision: StateRevision,
    pub core: CoreRulesState,
    pub zones: ZoneState,
    pub allocators: IdentityAllocatorState,
    pub execution: ExecutionState,
    pub random: RandomStateV1,
    pub knowledge: KnowledgeStateV2,
    pub perspective_identities: PerspectiveIdentityStateV2,
    pub format: FormatState,
}

impl EngineState {
    pub fn canonical_digest_bytes(&self) -> Result<Vec<u8>, StateDigestError> {
        crate::digest_v3::full_state_digest_input(self)?.canonical_payload()
    }

    pub fn digest(&self) -> Result<FullStateDigestV3, StateDigestError> {
        crate::digest_v3::calculate_full_state_digest_v3_for_state(self)
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
    pub knowledge: KnowledgeStateV2,
    pub perspective_identities: PerspectiveIdentityStateV2,
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
