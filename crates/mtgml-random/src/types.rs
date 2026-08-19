use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use thiserror::Error;

pub const MTGML_RNG_V1: &str = "mtgml.rng.v1";

pub fn validate_seed_hex(value: &str) -> Result<(), RandomValidationError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(RandomValidationError::InvalidSeedHex);
    }
    Ok(())
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RandomStreamKindV1 {
    SyntheticM1 = 0,
    Shuffle = 1,
    Sampling = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RandomStreamScopeV1 {
    Global = 0,
    Player = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RandomStreamKeyV1 {
    pub version: u8,
    pub kind: RandomStreamKindV1,
    pub scope: RandomStreamScopeV1,
    pub player: Option<u64>,
}

impl RandomStreamKeyV1 {
    pub const VERSION: u8 = 1;

    pub fn global(kind: RandomStreamKindV1) -> Self {
        Self {
            version: Self::VERSION,
            kind,
            scope: RandomStreamScopeV1::Global,
            player: None,
        }
    }

    pub fn player(kind: RandomStreamKindV1, player_id: u64) -> Self {
        Self {
            version: Self::VERSION,
            kind,
            scope: RandomStreamScopeV1::Player,
            player: Some(player_id),
        }
    }

    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(4 + 2 + 1 + 8);
        bytes.push(self.version);
        bytes.extend_from_slice(&self.kind.to_canonical_u16());
        bytes.push(self.scope.to_canonical_u8());
        match self.scope {
            RandomStreamScopeV1::Global => {}
            RandomStreamScopeV1::Player => {
                if let Some(p) = self.player {
                    bytes.extend_from_slice(&p.to_be_bytes());
                }
            }
        }
        bytes
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, RandomValidationError> {
        if bytes.is_empty() {
            return Err(RandomValidationError::MalformedStreamKey);
        }
        let version = bytes[0];
        if version != Self::VERSION {
            return Err(RandomValidationError::UnknownKeyVersion(version));
        }
        if bytes.len() < 4 {
            return Err(RandomValidationError::MalformedStreamKey);
        }
        let kind_u16 = u16::from_be_bytes([bytes[1], bytes[2]]);
        let scope_u8 = bytes[3];
        let kind = RandomStreamKindV1::from_canonical_u16(kind_u16)?;
        let scope = RandomStreamScopeV1::from_canonical_u8(scope_u8)?;
        let player = match scope {
            RandomStreamScopeV1::Global => {
                if bytes.len() != 4 {
                    return Err(RandomValidationError::MalformedStreamKey);
                }
                None
            }
            RandomStreamScopeV1::Player => {
                if bytes.len() != 12 {
                    return Err(RandomValidationError::MalformedStreamKey);
                }
                let p = u64::from_be_bytes([
                    bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10],
                    bytes[11],
                ]);
                Some(p)
            }
        };
        Ok(Self {
            version,
            kind,
            scope,
            player,
        })
    }
}

impl RandomStreamKindV1 {
    fn to_canonical_u16(self) -> [u8; 2] {
        (self as u16).to_be_bytes()
    }

    fn from_canonical_u16(u: u16) -> Result<Self, RandomValidationError> {
        match u {
            0 => Ok(Self::SyntheticM1),
            1 => Ok(Self::Shuffle),
            2 => Ok(Self::Sampling),
            _ => Err(RandomValidationError::UnknownKind(u)),
        }
    }
}

impl RandomStreamScopeV1 {
    fn to_canonical_u8(self) -> u8 {
        self as u8
    }

    fn from_canonical_u8(u: u8) -> Result<Self, RandomValidationError> {
        match u {
            0 => Ok(Self::Global),
            1 => Ok(Self::Player),
            _ => Err(RandomValidationError::UnknownScopeTag(u)),
        }
    }
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct RandomStreamCursorV1 {
    pub next_raw_u64: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RandomStateV1 {
    pub contract_id: String,
    pub root_seed: RootSeed256,
    #[serde(rename = "streams", serialize_with = "serialize_stream_entries")]
    pub streams: BTreeMap<RandomStreamKeyV1, RandomStreamCursorV1>,
}

fn serialize_stream_entries<S: serde::Serializer>(
    map: &BTreeMap<RandomStreamKeyV1, RandomStreamCursorV1>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let entries: Vec<CanonicalRandomStreamEntryV1> = map
        .iter()
        .map(|(key, cursor)| CanonicalRandomStreamEntryV1 {
            key: *key,
            next_raw_u64: cursor.next_raw_u64,
        })
        .collect();
    entries.serialize(serializer)
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
            if streams.contains_key(&entry.key) {
                return Err(serde::de::Error::custom(format!(
                    "duplicate stream key {:?}",
                    entry.key
                )));
            }
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
            streams: BTreeMap::new(),
        }
    }
}

impl Default for RandomStateV1 {
    fn default() -> Self {
        Self::new(RootSeed256::from_lower_hex(&"00".repeat(32)).unwrap())
    }
}

impl RandomStateV1 {
    pub fn add_stream(
        &mut self,
        key: RandomStreamKeyV1,
        cursor: RandomStreamCursorV1,
    ) -> Result<(), RandomValidationError> {
        if self.streams.contains_key(&key) {
            return Err(RandomValidationError::DuplicateStreamKey);
        }
        self.streams.insert(key, cursor);
        Ok(())
    }

    pub fn from_entries(
        root_seed: RootSeed256,
        entries: Vec<CanonicalRandomStreamEntryV1>,
    ) -> Result<Self, RandomValidationError> {
        let mut streams = BTreeMap::new();
        for entry in &entries {
            let key_bytes = entry.key.to_canonical_bytes();
            if streams.contains_key(&entry.key) {
                return Err(RandomValidationError::DuplicateStreamKey);
            }
            if entry.next_raw_u64 != 0 {
                return Err(RandomValidationError::NonZeroInitialCursor);
            }
            streams.insert(entry.key, RandomStreamCursorV1::default());
            drop(key_bytes);
        }
        Ok(Self {
            contract_id: MTGML_RNG_V1.to_owned(),
            root_seed,
            streams,
        })
    }

    pub fn validate(&self) -> Result<(), RandomValidationError> {
        if self.contract_id != MTGML_RNG_V1 {
            return Err(RandomValidationError::UnsupportedRngContract);
        }
        if self.streams.len() > u32::MAX as usize {
            return Err(RandomValidationError::TooManyStreams);
        }
        let mut prev_key_bytes: Option<Vec<u8>> = None;
        let mut seen_keys = std::collections::BTreeSet::new();
        for key in self.streams.keys() {
            let key_bytes = key.to_canonical_bytes();
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

    pub fn lookup_stream(
        &self,
        key: &RandomStreamKeyV1,
    ) -> Result<RandomStreamCursorV1, RandomValidationError> {
        self.streams
            .get(key)
            .copied()
            .ok_or(RandomValidationError::StreamNotFound)
    }

    pub fn set_cursor(
        &mut self,
        key: &RandomStreamKeyV1,
        cursor: RandomStreamCursorV1,
    ) -> Result<(), RandomValidationError> {
        self.streams
            .get_mut(key)
            .map(|c| *c = cursor)
            .ok_or(RandomValidationError::StreamNotFound)
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
    #[error("non-zero initial cursor in canonical entry")]
    NonZeroInitialCursor,
    #[error("too many streams")]
    TooManyStreams,
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_ZERO_SEED: &str = "0000000000000000000000000000000000000000000000000000000000000000";

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
    fn stream_key_global_canonical_roundtrip() {
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::Shuffle);
        let bytes = key.to_canonical_bytes();
        let parsed = RandomStreamKeyV1::from_canonical_bytes(&bytes).unwrap();
        assert_eq!(key, parsed);
        assert_eq!(bytes, parsed.to_canonical_bytes());
    }

    #[test]
    fn stream_key_player_canonical_roundtrip() {
        let key = RandomStreamKeyV1::player(RandomStreamKindV1::Sampling, 42);
        let bytes = key.to_canonical_bytes();
        let parsed = RandomStreamKeyV1::from_canonical_bytes(&bytes).unwrap();
        assert_eq!(key, parsed);
        assert_eq!(bytes, parsed.to_canonical_bytes());
    }

    #[test]
    fn stream_key_rejects_wrong_version() {
        let mut key = RandomStreamKeyV1::global(RandomStreamKindV1::Shuffle);
        key.version = 2;
        let bytes = key.to_canonical_bytes();
        assert!(RandomStreamKeyV1::from_canonical_bytes(&bytes).is_err());
    }

    #[test]
    fn stream_key_rejects_unknown_kind() {
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::Shuffle);
        let mut bytes = key.to_canonical_bytes();
        bytes[2] = 99;
        assert!(RandomStreamKeyV1::from_canonical_bytes(&bytes).is_err());
    }

    #[test]
    fn stream_key_rejects_unknown_scope() {
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::Shuffle);
        let mut bytes = key.to_canonical_bytes();
        bytes[3] = 99;
        assert!(RandomStreamKeyV1::from_canonical_bytes(&bytes).is_err());
    }

    #[test]
    fn stream_key_rejects_malformed_player() {
        let key = RandomStreamKeyV1::player(RandomStreamKindV1::Sampling, 42);
        let mut bytes = key.to_canonical_bytes();
        bytes.truncate(6);
        assert!(RandomStreamKeyV1::from_canonical_bytes(&bytes).is_err());
    }

    #[test]
    fn cursor_defaults_to_zero() {
        let cursor = RandomStreamCursorV1::default();
        assert_eq!(cursor.next_raw_u64, 0);
    }

    #[test]
    fn canonical_entries_sorted_by_key_bytes() {
        let p1 = RandomStreamKeyV1::player(RandomStreamKindV1::Shuffle, 1);
        let p2 = RandomStreamKeyV1::player(RandomStreamKindV1::Shuffle, 2);
        let global = RandomStreamKeyV1::global(RandomStreamKindV1::Shuffle);
        let mut map = BTreeMap::new();
        map.insert(p2, RandomStreamCursorV1::default());
        map.insert(global, RandomStreamCursorV1::default());
        map.insert(p1, RandomStreamCursorV1::default());
        let entries: Vec<_> = map
            .iter()
            .map(|(k, v)| CanonicalRandomStreamEntryV1 {
                key: *k,
                next_raw_u64: v.next_raw_u64,
            })
            .collect();
        let mut prev: Option<Vec<u8>> = None;
        for e in &entries {
            let b = e.key.to_canonical_bytes();
            if let Some(p) = &prev {
                assert!(p <= &b, "entries must be sorted by key bytes");
            }
            prev = Some(b);
        }
    }

    #[test]
    fn duplicate_stream_key_rejected() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let mut state = RandomStateV1::new(seed);
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::Shuffle);
        state
            .add_stream(key, RandomStreamCursorV1::default())
            .unwrap();
        assert_eq!(
            state.add_stream(key, RandomStreamCursorV1::default()),
            Err(RandomValidationError::DuplicateStreamKey)
        );
    }

    #[test]
    fn duplicate_deserialize_rejected() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::Shuffle);
        let entries = vec![
            CanonicalRandomStreamEntryV1 {
                key,
                next_raw_u64: 0,
            },
            CanonicalRandomStreamEntryV1 {
                key,
                next_raw_u64: 0,
            },
        ];
        assert_eq!(
            RandomStateV1::from_entries(seed, entries),
            Err(RandomValidationError::DuplicateStreamKey)
        );
    }

    #[test]
    fn non_zero_cursor_rejected() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let entries = vec![CanonicalRandomStreamEntryV1 {
            key: RandomStreamKeyV1::global(RandomStreamKindV1::Shuffle),
            next_raw_u64: 1,
        }];
        assert_eq!(
            RandomStateV1::from_entries(seed, entries),
            Err(RandomValidationError::NonZeroInitialCursor)
        );
    }

    #[test]
    fn serde_roundtrip_preserves_entries() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let mut state = RandomStateV1::new(seed);
        state
            .add_stream(
                RandomStreamKeyV1::global(RandomStreamKindV1::Shuffle),
                RandomStreamCursorV1 { next_raw_u64: 42 },
            )
            .unwrap();
        state
            .add_stream(
                RandomStreamKeyV1::player(RandomStreamKindV1::Sampling, 1),
                RandomStreamCursorV1 { next_raw_u64: 17 },
            )
            .unwrap();
        let json = serde_json::to_string(&state).unwrap();
        let parsed: RandomStateV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(state.contract_id, parsed.contract_id);
        assert_eq!(state.root_seed, parsed.root_seed);
        assert_eq!(state.streams.len(), parsed.streams.len());
        for (k, v) in &state.streams {
            assert_eq!(parsed.streams.get(k).unwrap(), v);
        }
    }

    #[test]
    fn cursor_mutation_changes_digest() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let mut state = RandomStateV1::new(seed);
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::Shuffle);
        state
            .add_stream(key, RandomStreamCursorV1::default())
            .unwrap();
        let json1 = serde_json::to_string(&state).unwrap();
        state
            .set_cursor(&key, RandomStreamCursorV1 { next_raw_u64: 1 })
            .unwrap();
        let json2 = serde_json::to_string(&state).unwrap();
        assert_ne!(json1, json2, "cursor mutation must change serialized form");
    }

    #[test]
    fn insertion_order_does_not_change_digest() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let k1 = RandomStreamKeyV1::global(RandomStreamKindV1::Shuffle);
        let k2 = RandomStreamKeyV1::player(RandomStreamKindV1::Sampling, 1);
        let mut state1 = RandomStateV1::new(seed);
        state1
            .add_stream(k1, RandomStreamCursorV1::default())
            .unwrap();
        state1
            .add_stream(k2, RandomStreamCursorV1::default())
            .unwrap();
        let json1 = serde_json::to_string(&state1).unwrap();

        let mut state2 = RandomStateV1::new(seed);
        state2
            .add_stream(k2, RandomStreamCursorV1::default())
            .unwrap();
        state2
            .add_stream(k1, RandomStreamCursorV1::default())
            .unwrap();
        let json2 = serde_json::to_string(&state2).unwrap();
        assert_eq!(
            json1, json2,
            "canonical order must be independent of insertion order"
        );
    }

    #[test]
    fn lookup_stream_success() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let mut state = RandomStateV1::new(seed);
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::Shuffle);
        state
            .add_stream(key, RandomStreamCursorV1 { next_raw_u64: 5 })
            .unwrap();
        let cursor = state.lookup_stream(&key).unwrap();
        assert_eq!(cursor.next_raw_u64, 5);
    }

    #[test]
    fn lookup_stream_missing_returns_error() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let state = RandomStateV1::new(seed);
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::Shuffle);
        assert_eq!(
            state.lookup_stream(&key),
            Err(RandomValidationError::StreamNotFound)
        );
    }

    #[test]
    fn set_cursor_updates_existing() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let mut state = RandomStateV1::new(seed);
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::Shuffle);
        state
            .add_stream(key, RandomStreamCursorV1::default())
            .unwrap();
        state
            .set_cursor(&key, RandomStreamCursorV1 { next_raw_u64: 99 })
            .unwrap();
        assert_eq!(state.lookup_stream(&key).unwrap().next_raw_u64, 99);
    }

    #[test]
    fn set_cursor_missing_returns_error() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let mut state = RandomStateV1::new(seed);
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::Shuffle);
        assert_eq!(
            state.set_cursor(&key, RandomStreamCursorV1::default()),
            Err(RandomValidationError::StreamNotFound)
        );
    }
}
