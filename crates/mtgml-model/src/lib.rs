//! Shared, rules-neutral identifiers and closed public status types.

use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};
use std::{fmt, str::FromStr};
use thiserror::Error;

mod generated_contract_vocab;
pub use generated_contract_vocab::{
    PlayerResult, TerminalReason, TruncationReason, ZoneKind, STABLE_WIRE_ERROR_CODES,
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

domain_digest!(FullStateDigest, "mtgml.full-state-digest.v1");
domain_digest!(PublicStateDigest, "mtgml.public-state-digest.v1");
domain_digest!(InformationStateDigest, "mtgml.information-state-digest.v1");
domain_digest!(ObservationDigest, "mtgml.observation-digest.v1");
domain_digest!(CandidateSetDigest, "mtgml.candidate-set-digest.v1");
domain_digest!(ContentDigest, "mtgml.content-digest.v1");
domain_digest!(ReplayDigest, "mtgml.replay-digest.v1");
domain_digest!(CheckpointDigest, "mtgml.checkpoint-digest.v1");

// === V2 digest domains ===
domain_digest!(FullStateDigestV2, "mtgml.full-state-digest.v2");
domain_digest!(CheckpointDigestV2, "mtgml.checkpoint-digest.v2");

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
    fn digest_encoding_matches_stable_sha256_vectors() {
        assert_eq!(
            Digest::from_bytes(b"").as_str(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        let bytes = b"same canonical bytes";
        assert_eq!(
            FullStateDigest::from_canonical_bytes(bytes).as_str(),
            "753d6b2756e60227af224f919169ce034dd94c60b142f6eded0931548d866c1a"
        );
        assert_eq!(
            PublicStateDigest::from_canonical_bytes(bytes).as_str(),
            "9ba50ec6cfe0ac09da1ce7844c3362bf8ea7643936a8384835d8718a3d25e442"
        );
        assert_eq!(
            InformationStateDigest::from_canonical_bytes(bytes).as_str(),
            "1bcb220ae39f043426dedf86a5422f95cc0a61cbeb3690efa4d853f0b0986a4c"
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
            CheckpointDigest::from_canonical_bytes(bytes).as_str(),
            "e68abf91d68057ab81bcbacdbc9f1dfe35b4e3fded6b9f8309fb7c948d8750f4"
        );
    }

    #[test]
    fn digest_domains_cannot_compare_accidentally_and_hash_differently() {
        let full = FullStateDigest::from_canonical_bytes(b"same canonical bytes");
        let public = PublicStateDigest::from_canonical_bytes(b"same canonical bytes");
        assert_ne!(full.as_str(), public.as_str());
        assert_eq!(FullStateDigest::parse(full.as_str()).unwrap(), full);
    }
}
