use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use thiserror::Error;

pub const MTGML_RNG_V1: &str = "mtgml.rng.v1";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RootSeed256(pub [u8; 32]);

impl RootSeed256 {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn from_lower_hex(hex: &str) -> Result<Self, RandomValidationError> {
        if hex.len() != 64 {
            return Err(RandomValidationError::InvalidSeedHex);
        }
        let bytes = hex.as_bytes();
        let mut result = [0u8; 32];
        for i in 0..32 {
            let hi = hex_byte(bytes[i * 2])?;
            let lo = hex_byte(bytes[i * 2 + 1])?;
            result[i] = (hi << 4) | lo;
        }
        Ok(Self(result))
    }

    pub fn to_lower_hex(&self) -> String {
        encode_lower_hex(&self.0)
    }
}

impl fmt::Debug for RootSeed256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RootSeed256(***)")
    }
}

impl Serialize for RootSeed256 {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_lower_hex())
    }
}

impl<'de> Deserialize<'de> for RootSeed256 {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        RootSeed256::from_lower_hex(&text).map_err(serde::de::Error::custom)
    }
}

fn hex_byte(ch: u8) -> Result<u8, RandomValidationError> {
    match ch {
        b'0'..=b'9' => Ok(ch - b'0'),
        b'a'..=b'f' => Ok(ch - b'a' + 10),
        _ => Err(RandomValidationError::InvalidSeedHex),
    }
}

pub fn encode_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RandomStreamKindV1 {
    SyntheticM1,
}

impl RandomStreamKindV1 {
    pub fn code(self) -> u16 {
        match self {
            Self::SyntheticM1 => 0x0001,
        }
    }

    pub fn from_code(code: u16) -> Option<Self> {
        match code {
            0x0001 => Some(Self::SyntheticM1),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RandomStreamScopeV1 {
    Global,
    Player(mtgml_model::PlayerId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RandomStreamKeyV1 {
    pub kind: RandomStreamKindV1,
    pub scope: RandomStreamScopeV1,
}

impl RandomStreamKeyV1 {
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(12);
        bytes.push(0x01);
        bytes.extend_from_slice(&self.kind.code().to_be_bytes());
        match self.scope {
            RandomStreamScopeV1::Global => {
                bytes.push(0x00);
            }
            RandomStreamScopeV1::Player(player) => {
                bytes.push(0x01);
                bytes.extend_from_slice(&player.0.to_be_bytes());
            }
        }
        bytes
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, RandomValidationError> {
        if bytes.is_empty() {
            return Err(RandomValidationError::MalformedStreamKey);
        }
        if bytes[0] != 0x01 {
            return Err(RandomValidationError::UnknownKeyVersion(bytes[0]));
        }
        if bytes.len() < 4 {
            return Err(RandomValidationError::MalformedStreamKey);
        }
        let kind_code = u16::from_be_bytes([bytes[1], bytes[2]]);
        let kind =
            RandomStreamKindV1::from_code(kind_code).ok_or(RandomValidationError::UnknownKind(
                kind_code,
            ))?;
        let scope_tag = bytes[3];
        let scope = match scope_tag {
            0x00 => {
                if bytes.len() != 4 {
                    return Err(RandomValidationError::MalformedStreamKey);
                }
                RandomStreamScopeV1::Global
            }
            0x01 => {
                if bytes.len() != 12 {
                    return Err(RandomValidationError::MalformedStreamKey);
                }
                let player_id = u64::from_be_bytes([
                    bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11],
                ]);
                RandomStreamScopeV1::Player(mtgml_model::PlayerId(player_id))
            }
            _ => return Err(RandomValidationError::UnknownScopeTag(scope_tag)),
        };
        Ok(Self { kind, scope })
    }
}

impl Serialize for RandomStreamKeyV1 {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct KeyDto<'a> {
            kind: &'a RandomStreamKindV1,
            scope: &'a RandomStreamScopeV1,
        }
        KeyDto {
            kind: &self.kind,
            scope: &self.scope,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RandomStreamKeyV1 {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct KeyDto {
            kind: RandomStreamKindV1,
            scope: RandomStreamScopeV1,
        }
        let dto = KeyDto::deserialize(deserializer)?;
        Ok(Self {
            kind: dto.kind,
            scope: dto.scope,
        })
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct RandomStreamCursorV1 {
    pub next_raw_u64: u64,
}

impl Default for RandomStreamCursorV1 {
    fn default() -> Self {
        Self { next_raw_u64: 0 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RandomStateV1 {
    pub contract_id: String,
    pub root_seed: RootSeed256,
    #[serde(rename = "streams")]
    pub stream_entries: Vec<CanonicalRandomStreamEntryV1>,
    #[serde(skip)]
    pub streams: BTreeMap<RandomStreamKeyV1, RandomStreamCursorV1>,
}

impl<'de> Deserialize<'de> for RandomStateV1 {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct RandomStateDto {
            contract_id: String,
            root_seed: RootSeed256,
            #[serde(rename = "streams")]
            stream_entries: Vec<CanonicalRandomStreamEntryV1>,
        }
        let dto = RandomStateDto::deserialize(deserializer)?;
        let mut streams = BTreeMap::new();
        for entry in &dto.stream_entries {
            streams.insert(
                entry.key,
                RandomStreamCursorV1 {
                    next_raw_u64: entry.next_raw_u64,
                },
            );
        }
        Ok(Self {
            contract_id: dto.contract_id,
            root_seed: dto.root_seed,
            stream_entries: dto.stream_entries,
            streams,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalRandomStreamEntryV1 {
    pub key: RandomStreamKeyV1,
    pub next_raw_u64: u64,
}

impl RandomStateV1 {
    pub fn new(root_seed: RootSeed256) -> Self {
        Self {
            contract_id: MTGML_RNG_V1.to_owned(),
            root_seed,
            stream_entries: Vec::new(),
            streams: BTreeMap::new(),
        }
    }

    pub fn add_stream(
        &mut self,
        key: RandomStreamKeyV1,
        cursor: RandomStreamCursorV1,
    ) -> Result<(), RandomValidationError> {
        if self.streams.contains_key(&key) {
            return Err(RandomValidationError::DuplicateStreamKey);
        }
        self.streams.insert(key, cursor);
        self.rebuild_entries();
        Ok(())
    }

    fn rebuild_entries(&mut self) {
        let mut entries: Vec<_> = self
            .streams
            .iter()
            .map(|(key, cursor)| CanonicalRandomStreamEntryV1 {
                key: *key,
                next_raw_u64: cursor.next_raw_u64,
            })
            .collect();
        entries.sort_by(|a, b| a.key.to_canonical_bytes().cmp(&b.key.to_canonical_bytes()));
        self.stream_entries = entries;
    }

    pub fn from_entries(
        root_seed: RootSeed256,
        entries: Vec<CanonicalRandomStreamEntryV1>,
    ) -> Result<Self, RandomValidationError> {
        let mut streams = BTreeMap::new();
        for entry in &entries {
            let key_bytes = entry.key.to_canonical_bytes();
            if streams
                .keys()
                .any(|k: &RandomStreamKeyV1| k.to_canonical_bytes() == key_bytes)
            {
                return Err(RandomValidationError::DuplicateStreamKey);
            }
            streams.insert(
                entry.key,
                RandomStreamCursorV1 {
                    next_raw_u64: entry.next_raw_u64,
                },
            );
        }
        let mut sorted = entries;
        sorted.sort_by(|a, b| a.key.to_canonical_bytes().cmp(&b.key.to_canonical_bytes()));
        Ok(Self {
            contract_id: MTGML_RNG_V1.to_owned(),
            root_seed,
            stream_entries: sorted,
            streams,
        })
    }

    pub fn validate(&self) -> Result<(), RandomValidationError> {
        if self.contract_id != MTGML_RNG_V1 {
            return Err(RandomValidationError::UnsupportedRngContract);
        }
        if self.stream_entries.len() != self.streams.len() {
            return Err(RandomValidationError::StreamEntryMismatch);
        }
        let mut prev_key_bytes: Option<Vec<u8>> = None;
        let mut seen_keys = std::collections::BTreeSet::new();
        for entry in &self.stream_entries {
            let key_bytes = entry.key.to_canonical_bytes();
            if !seen_keys.insert(key_bytes.clone()) {
                return Err(RandomValidationError::DuplicateStreamKey);
            }
            if let Some(prev) = &prev_key_bytes {
                if prev > &key_bytes {
                    return Err(RandomValidationError::UnorderedStreamEntries);
                }
            }
            prev_key_bytes = Some(key_bytes);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RandomValidationError {
    #[error("unsupported RNG contract")]
    UnsupportedRngContract,
    #[error("root seed must be exactly 64 lowercase hexadecimal characters")]
    InvalidSeedHex,
    #[error("unknown stream-key codec version: {0}")]
    UnknownKeyVersion(u8),
    #[error("unknown stream kind code: {0}")]
    UnknownKind(u16),
    #[error("unknown scope tag: {0}")]
    UnknownScopeTag(u8),
    #[error("malformed stream key bytes")]
    MalformedStreamKey,
    #[error("duplicate stream key")]
    DuplicateStreamKey,
    #[error("stream entries do not match stream map")]
    StreamEntryMismatch,
    #[error("stream entries are not in canonical key-byte order")]
    UnorderedStreamEntries,
    #[error("player-scoped stream references an absent player")]
    PlayerScopeMismatch,
    #[error("invalid random bound")]
    InvalidRandomBound,
    #[error("random stream exhausted")]
    StreamExhausted,
    #[error("requested stream not found")]
    StreamNotFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_ZERO_SEED: &str =
        "0000000000000000000000000000000000000000000000000000000000000000";

    #[test]
    fn root_seed_roundtrip() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        assert_eq!(seed.to_lower_hex(), ALL_ZERO_SEED);
        assert_eq!(seed.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn root_seed_rejects_wrong_length() {
        assert!(RootSeed256::from_lower_hex("00").is_err());
        assert!(RootSeed256::from_lower_hex(&"a".repeat(65)).is_err());
    }

    #[test]
    fn root_seed_rejects_uppercase() {
        let mut hex = ALL_ZERO_SEED.to_owned();
        hex.replace_range(..1, "A");
        assert!(RootSeed256::from_lower_hex(&hex).is_err());
    }

    #[test]
    fn root_seed_rejects_nonhex() {
        let mut hex = ALL_ZERO_SEED.to_owned();
        hex.replace_range(..1, "g");
        assert!(RootSeed256::from_lower_hex(&hex).is_err());
    }

    #[test]
    fn global_key_canonical_bytes() {
        let key = RandomStreamKeyV1 {
            kind: RandomStreamKindV1::SyntheticM1,
            scope: RandomStreamScopeV1::Global,
        };
        assert_eq!(key.to_canonical_bytes(), vec![0x01, 0x00, 0x01, 0x00]);
    }

    #[test]
    fn player_key_canonical_bytes() {
        let key = RandomStreamKeyV1 {
            kind: RandomStreamKindV1::SyntheticM1,
            scope: RandomStreamScopeV1::Player(mtgml_model::PlayerId(1)),
        };
        let bytes = key.to_canonical_bytes();
        assert_eq!(
            bytes,
            vec![0x01, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]
        );
    }

    #[test]
    fn key_roundtrip() {
        let key = RandomStreamKeyV1 {
            kind: RandomStreamKindV1::SyntheticM1,
            scope: RandomStreamScopeV1::Global,
        };
        let decoded = RandomStreamKeyV1::from_canonical_bytes(&key.to_canonical_bytes()).unwrap();
        assert_eq!(key, decoded);
    }

    #[test]
    fn key_rejects_unknown_version() {
        let bytes = vec![0x02, 0x00, 0x01, 0x00];
        assert!(matches!(
            RandomStreamKeyV1::from_canonical_bytes(&bytes),
            Err(RandomValidationError::UnknownKeyVersion(0x02))
        ));
    }

    #[test]
    fn key_rejects_unknown_kind() {
        let bytes = vec![0x01, 0x00, 0x02, 0x00];
        assert!(matches!(
            RandomStreamKeyV1::from_canonical_bytes(&bytes),
            Err(RandomValidationError::UnknownKind(0x0002))
        ));
    }

    #[test]
    fn key_rejects_unknown_scope() {
        let bytes = vec![0x01, 0x00, 0x01, 0x02];
        assert!(matches!(
            RandomStreamKeyV1::from_canonical_bytes(&bytes),
            Err(RandomValidationError::UnknownScopeTag(0x02))
        ));
    }

    #[test]
    fn key_rejects_truncated_player() {
        let bytes = vec![0x01, 0x00, 0x01, 0x01, 0x00, 0x01];
        assert!(RandomStreamKeyV1::from_canonical_bytes(&bytes).is_err());
    }

    #[test]
    fn key_rejects_trailing_bytes() {
        let bytes = vec![0x01, 0x00, 0x01, 0x00, 0x00];
        assert!(RandomStreamKeyV1::from_canonical_bytes(&bytes).is_err());
    }

    #[test]
    fn cursor_defaults_to_zero() {
        let cursor = RandomStreamCursorV1::default();
        assert_eq!(cursor.next_raw_u64, 0);
    }

    #[test]
    fn random_state_new_has_zero_streams() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let state = RandomStateV1::new(seed);
        assert_eq!(state.contract_id, MTGML_RNG_V1);
        assert!(state.streams.is_empty());
        state.validate().unwrap();
    }

    #[test]
    fn random_state_rejects_wrong_contract_id() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let mut state = RandomStateV1::new(seed);
        state.contract_id = "wrong".into();
        assert_eq!(
            state.validate(),
            Err(RandomValidationError::UnsupportedRngContract)
        );
    }

    #[test]
    fn canonical_entries_sorted_by_key_bytes() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let mut state = RandomStateV1::new(seed);
        let player_key = RandomStreamKeyV1 {
            kind: RandomStreamKindV1::SyntheticM1,
            scope: RandomStreamScopeV1::Player(mtgml_model::PlayerId(1)),
        };
        let global_key = RandomStreamKeyV1 {
            kind: RandomStreamKindV1::SyntheticM1,
            scope: RandomStreamScopeV1::Global,
        };
        state.add_stream(player_key, RandomStreamCursorV1::default()).unwrap();
        state.add_stream(global_key, RandomStreamCursorV1::default()).unwrap();
        assert_eq!(state.stream_entries.len(), 2);
        let key0_bytes = state.stream_entries[0].key.to_canonical_bytes();
        let key1_bytes = state.stream_entries[1].key.to_canonical_bytes();
        assert!(key0_bytes < key1_bytes);
    }

    #[test]
    fn duplicate_stream_key_rejected() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let mut state = RandomStateV1::new(seed);
        let key = RandomStreamKeyV1 {
            kind: RandomStreamKindV1::SyntheticM1,
            scope: RandomStreamScopeV1::Global,
        };
        state.add_stream(key, RandomStreamCursorV1::default()).unwrap();
        assert_eq!(
            state.add_stream(key, RandomStreamCursorV1::default()),
            Err(RandomValidationError::DuplicateStreamKey)
        );
    }

    #[test]
    fn from_entries_rejects_duplicate_keys() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let key = RandomStreamKeyV1 {
            kind: RandomStreamKindV1::SyntheticM1,
            scope: RandomStreamScopeV1::Global,
        };
        let entries = vec![
            CanonicalRandomStreamEntryV1 {
                key,
                next_raw_u64: 0,
            },
            CanonicalRandomStreamEntryV1 {
                key,
                next_raw_u64: 1,
            },
        ];
        assert_eq!(
            RandomStateV1::from_entries(seed, entries),
            Err(RandomValidationError::DuplicateStreamKey)
        );
    }

    #[test]
    fn serde_roundtrip_via_entries() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let mut state = RandomStateV1::new(seed);
        let key = RandomStreamKeyV1 {
            kind: RandomStreamKindV1::SyntheticM1,
            scope: RandomStreamScopeV1::Global,
        };
        state.add_stream(key, RandomStreamCursorV1 { next_raw_u64: 5 }).unwrap();
        let json = serde_json::to_string(&state).unwrap();
        let decoded: RandomStateV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.root_seed, state.root_seed);
        assert_eq!(decoded.streams.len(), 1);
        assert_eq!(
            decoded.streams[&key],
            RandomStreamCursorV1 { next_raw_u64: 5 }
        );
    }
}
