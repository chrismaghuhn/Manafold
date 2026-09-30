//! Shared, rules-neutral identifiers and closed public status types.

use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};
use std::{fmt, str::FromStr};
use thiserror::Error;

mod generated_contract_vocab;
pub use generated_contract_vocab::{
    PlayerResult, TerminalReason, TruncationReason, ZoneKind, STABLE_WIRE_ERROR_CODES,
};

mod execution_identity;
mod semantic_contract;
pub use execution_identity::{
    execution_program_matches_rules_authority, ExecutionIdentityV1, ExecutionProgramV1,
};
pub use semantic_contract::{
    CapabilityRequirementV1, RulesAuthorityV1, RulesContractManifestV1,
    RulesContractManifestValidationError, SemanticContractManifestV1,
};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CanonicalIntegerError {
    #[error("integer text is empty")]
    Empty,
    #[error("integer text is not canonical unsigned decimal")]
    NonCanonical,
    #[error("integer exceeds the supported range")]
    OutOfRange,
}

pub fn parse_canonical_u64(text: &str) -> Result<u64, CanonicalIntegerError> {
    if text.is_empty() {
        return Err(CanonicalIntegerError::Empty);
    }
    if text != "0" && text.starts_with('0') {
        return Err(CanonicalIntegerError::NonCanonical);
    }
    if !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(CanonicalIntegerError::NonCanonical);
    }
    text.parse::<u64>()
        .map_err(|_| CanonicalIntegerError::OutOfRange)
}

macro_rules! canonical_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        pub struct $name(pub u64);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = CanonicalIntegerError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                parse_canonical_u64(value).map(Self)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.0.to_string())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let text = String::deserialize(deserializer)?;
                text.parse().map_err(D::Error::custom)
            }
        }
    };
}

canonical_id!(PlayerId);
canonical_id!(CardDefinitionId);
canonical_id!(PhysicalCardId);
canonical_id!(GameObjectId);
canonical_id!(AbilityInstanceId);
canonical_id!(StackObjectId);
canonical_id!(EffectInstanceId);
canonical_id!(TriggerInstanceId);
canonical_id!(DecisionId);
canonical_id!(ContinuationId);
canonical_id!(RuleEventId);
canonical_id!(OpaqueObjectId);
canonical_id!(OpaqueAbilityId);
canonical_id!(StateRevision);
canonical_id!(EventSequence);

fn encode_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest(String);

impl Digest {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        Self(encode_lower_hex(digest.as_ref()))
    }

    pub fn parse(text: impl Into<String>) -> Result<Self, DigestError> {
        let text = text.into();
        if text.len() != 64
            || !text
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(DigestError);
        }
        Ok(Self(text))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for Digest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        Self::parse(text).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("digest must contain exactly 64 lowercase hexadecimal characters")]
pub struct DigestError;

macro_rules! domain_digest {
    ($name:ident, $domain:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Digest);

        impl $name {
            pub const DOMAIN: &'static str = $domain;

            pub fn from_canonical_bytes(bytes: &[u8]) -> Self {
                let mut hasher = Sha256::new();
                hasher.update(Self::DOMAIN.as_bytes());
                hasher.update([0u8]);
                hasher.update(bytes);
                let digest = hasher.finalize();
                Self(Digest(encode_lower_hex(digest.as_ref())))
            }

            pub fn parse(text: impl Into<String>) -> Result<Self, DigestError> {
                Digest::parse(text).map(Self)
            }

            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            pub fn into_untyped(self) -> Digest {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                self.0.serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Digest::deserialize(deserializer).map(Self)
            }
        }
    };
}

domain_digest!(PublicStateDigest, "mtgml.public-state-digest.v1");
domain_digest!(ObservationDigest, "mtgml.observation-digest.v1");
domain_digest!(CandidateSetDigest, "mtgml.candidate-set-digest.v1");
domain_digest!(ContentDigest, "mtgml.content-digest.v1");
domain_digest!(ReplayDigest, "mtgml.replay-digest.v1");

// === V2 digest domains ===
domain_digest!(
    InformationStateDigestV2,
    "mtgml.information-state-digest.v2"
);
domain_digest!(
    InformationStateDigestV3,
    "mtgml.information-state-digest.v3"
);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct CandidateIdV1(pub u32);

impl fmt::Display for CandidateIdV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u32> for CandidateIdV1 {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

canonical_id!(PlayerDecisionIdV1);
canonical_id!(VisibleSequence);

fn decode_lower_hex_32(text: &str) -> Result<[u8; 32], DigestError> {
    if text.len() != 64
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(DigestError);
    }
    let mut bytes = [0u8; 32];
    for (index, chunk) in text.as_bytes().chunks_exact(2).enumerate() {
        let high = (chunk[0] as char).to_digit(16).ok_or(DigestError)? as u8;
        let low = (chunk[1] as char).to_digit(16).ok_or(DigestError)? as u8;
        bytes[index] = (high << 4) | low;
    }
    Ok(bytes)
}

macro_rules! raw_digest {
    ($name:ident, $domain:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Digest);

        impl $name {
            pub const DOMAIN: &'static str = $domain;

            pub fn from_digest_bytes(bytes: [u8; 32]) -> Self {
                Self(Digest(encode_lower_hex(&bytes)))
            }

            pub fn parse(text: impl Into<String>) -> Result<Self, DigestError> {
                let text = text.into();
                decode_lower_hex_32(&text)?;
                Ok(Self(Digest(text)))
            }

            pub fn raw_bytes(&self) -> [u8; 32] {
                decode_lower_hex_32(self.0.as_str()).expect("raw digest invariant")
            }

            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            pub fn into_untyped(self) -> Digest {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                self.0.serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Digest::deserialize(deserializer)
                    .and_then(|digest| Self::parse(digest.as_str()).map_err(D::Error::custom))
            }
        }
    };
}

raw_digest!(FullStateDigestV7, "mtgml.full-state-digest.v7");

// === V5 contract identity and digest domains (spec §5) ===
raw_digest!(RulesContractIdV1, "mtgml.rules-contract.v1");
raw_digest!(SemanticContractIdV1, "mtgml.semantic-contract.v1");
raw_digest!(CheckpointDigestV8, "mtgml.checkpoint-digest.v8");

/// Reserved digest identity newtype: DOMAIN + canonical hex parse/serde only.
///
/// Deliberately NO construction from arbitrary digest bytes (spec §5): the
/// reserved format/content contract identities cannot be minted before their
/// contracts exist. Values may only arrive via canonical 64-lowercase-hex
/// text (typed-seam decode); no arbitrary-byte construction or semantic
/// derivation path exists. Reading the raw digest bytes of an already-valid
/// value (`raw_bytes`) is an observation, not a minting path.
macro_rules! reserved_digest {
    ($name:ident, $domain:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Digest);

        impl $name {
            pub const DOMAIN: &'static str = $domain;

            pub fn parse(text: impl Into<String>) -> Result<Self, DigestError> {
                let text = text.into();
                decode_lower_hex_32(&text)?;
                Ok(Self(Digest(text)))
            }

            pub fn raw_bytes(&self) -> [u8; 32] {
                decode_lower_hex_32(self.0.as_str()).expect("reserved digest invariant")
            }

            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            pub fn into_untyped(self) -> Digest {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                self.0.serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Digest::deserialize(deserializer)
                    .and_then(|digest| Self::parse(digest.as_str()).map_err(D::Error::custom))
            }
        }
    };
}

reserved_digest!(FormatContractIdV1, "mtgml.format-contract.v1");
reserved_digest!(ContentContractIdV1, "mtgml.content-contract.v1");

impl FullStateDigestV7 {
    pub fn as_digest_reference(&self) -> DigestReferenceV1 {
        DigestReferenceV1 {
            envelope_version: "mtgml.digest-envelope.v1".to_owned(),
            algorithm_id: "sha-256".to_owned(),
            semantic_domain: Self::DOMAIN.to_owned(),
            payload_codec_id: "mtgml.canonical-cbor.v1".to_owned(),
            input_schema_id: "full-state-digest-input.v7".to_owned(),
            digest_bytes: self.raw_bytes(),
        }
    }
}

impl CheckpointDigestV8 {
    pub fn as_digest_reference(&self) -> DigestReferenceV1 {
        DigestReferenceV1 {
            envelope_version: "mtgml.digest-envelope.v1".to_owned(),
            algorithm_id: "sha-256".to_owned(),
            semantic_domain: Self::DOMAIN.to_owned(),
            payload_codec_id: "mtgml.canonical-cbor.v1".to_owned(),
            input_schema_id: "environment-checkpoint-digest-input.v8".to_owned(),
            digest_bytes: self.raw_bytes(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentLimitCounters {
    pub decisions_submitted: u64,
    pub accepted_transitions: u64,
    pub rule_events_emitted: u64,
    pub resource_units_consumed: u64,
    pub wall_clock_elapsed_millis: u64,
}

impl EnvironmentLimitCounters {
    pub fn validate(&self) -> Result<(), EnvironmentLimitCounterValidationError> {
        if self.accepted_transitions > self.decisions_submitted {
            return Err(EnvironmentLimitCounterValidationError::AcceptedTransitionsExceedDecisions);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum EnvironmentLimitCounterValidationError {
    #[error("accepted transitions exceed submitted decisions")]
    AcceptedTransitionsExceedDecisions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointCodecIdentity {
    pub codec_id: String,
    pub semantic_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestReferenceV1 {
    pub envelope_version: String,
    pub algorithm_id: String,
    pub semantic_domain: String,
    pub payload_codec_id: String,
    pub input_schema_id: String,
    pub digest_bytes: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerOutcome {
    pub player: PlayerId,
    pub result: PlayerResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EpisodeStatus {
    Running,
    Terminal {
        reason: TerminalReason,
        players: Vec<PlayerOutcome>,
    },
    Truncated {
        reason: TruncationReason,
        players: Vec<PlayerOutcome>,
    },
}

impl EpisodeStatus {
    pub fn validate(&self) -> Result<(), StatusValidationError> {
        let players = match self {
            Self::Running => return Ok(()),
            Self::Terminal { players, .. } | Self::Truncated { players, .. } => players,
        };
        let mut ids = std::collections::BTreeSet::new();
        if players.iter().any(|outcome| !ids.insert(outcome.player)) {
            return Err(StatusValidationError::DuplicatePlayer);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum StatusValidationError {
    #[error("episode status contains the same player more than once")]
    DuplicatePlayer,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_ids_reject_leading_zeroes() {
        assert!("01".parse::<PlayerId>().is_err());
        assert_eq!("0".parse::<PlayerId>().unwrap(), PlayerId(0));
    }

    #[test]
    fn episode_reasons_are_closed_during_deserialization() {
        let value = r#"{"kind":"terminal","reason":"banana","players":[]}"#;
        assert!(serde_json::from_str::<EpisodeStatus>(value).is_err());
    }

    #[test]
    fn environment_limit_counters_reject_impossible_acceptance_count() {
        let invalid = EnvironmentLimitCounters {
            accepted_transitions: 1,
            decisions_submitted: 0,
            ..EnvironmentLimitCounters::default()
        };
        assert_eq!(
            invalid.validate(),
            Err(EnvironmentLimitCounterValidationError::AcceptedTransitionsExceedDecisions)
        );
        assert!(EnvironmentLimitCounters::default().validate().is_ok());
    }

    #[test]
    fn digest_encoding_matches_stable_sha256_vectors() {
        assert_eq!(
            Digest::from_bytes(b"").as_str(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        let bytes = b"same canonical bytes";
        assert_eq!(
            PublicStateDigest::from_canonical_bytes(bytes).as_str(),
            "9ba50ec6cfe0ac09da1ce7844c3362bf8ea7643936a8384835d8718a3d25e442"
        );
        assert_eq!(
            ObservationDigest::from_canonical_bytes(bytes).as_str(),
            "9456c304ea3d6e0dd3637ae7b0c913869e31a0d29a8d2a22b6320e557449be03"
        );
        assert_eq!(
            CandidateSetDigest::from_canonical_bytes(bytes).as_str(),
            "1e74e9031e32e0adaab4c54df259744c6a0b20935e97e2356e7e18c352ff7eb5"
        );
        assert_eq!(
            ContentDigest::from_canonical_bytes(bytes).as_str(),
            "a5465c6197e67749768cb91df6d1e6f8831257b5cefbb35205a04bcfc9d04e01"
        );
        assert_eq!(
            ReplayDigest::from_canonical_bytes(bytes).as_str(),
            "93e08ec06c09ef5c53e0cc350c045176cc654c31cbef6afc08ad227fa9d413c8"
        );
        assert_eq!(
            InformationStateDigestV3::from_canonical_bytes(bytes).as_str(),
            "334e1a727bd9db7f920f6cf9c4e1a31e0c389fddc1b15e113d10bccd7bc626db"
        );
    }

    #[test]
    fn digest_domains_cannot_compare_accidentally_and_hash_differently() {
        let observation = ObservationDigest::from_canonical_bytes(b"same canonical bytes");
        let public = PublicStateDigest::from_canonical_bytes(b"same canonical bytes");
        assert_ne!(observation.as_str(), public.as_str());
        assert_eq!(
            ObservationDigest::parse(observation.as_str()).unwrap(),
            observation
        );
    }

    #[test]
    fn g0_digest_values_bind_the_allocated_v7_and_v8_domains() {
        let full = FullStateDigestV7::from_digest_bytes([0x71; 32]);
        let checkpoint = CheckpointDigestV8::from_digest_bytes([0x82; 32]);
        let full_reference = full.as_digest_reference();
        let checkpoint_reference = checkpoint.as_digest_reference();
        assert_eq!(full_reference.semantic_domain, "mtgml.full-state-digest.v7");
        assert_eq!(full_reference.input_schema_id, "full-state-digest-input.v7");
        assert_eq!(full_reference.digest_bytes, [0x71; 32]);
        assert_eq!(
            checkpoint_reference.semantic_domain,
            "mtgml.checkpoint-digest.v8"
        );
        assert_eq!(
            checkpoint_reference.input_schema_id,
            "environment-checkpoint-digest-input.v8"
        );
        assert_eq!(checkpoint_reference.digest_bytes, [0x82; 32]);
    }

    #[test]
    fn m2_b_candidate_id_is_u32_and_v3_digest_is_raw() {
        assert_eq!(std::mem::size_of::<CandidateIdV1>(), 4);
        assert_eq!(serde_json::to_string(&CandidateIdV1(7)).unwrap(), "7");
        assert_eq!(
            serde_json::to_string(&PlayerDecisionIdV1(7)).unwrap(),
            "\"7\""
        );
        assert_eq!(serde_json::to_string(&VisibleSequence(7)).unwrap(), "\"7\"");
        assert_eq!(
            FullStateDigestV7::from_digest_bytes([0xabu8; 32]).raw_bytes(),
            [0xabu8; 32]
        );
        assert!(serde_json::from_str::<CandidateIdV1>("4294967296").is_err());
        assert!(serde_json::from_str::<CandidateIdV1>("-1").is_err());
    }
}
