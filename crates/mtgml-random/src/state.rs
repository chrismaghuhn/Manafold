use crate::seed::{RandomValidationError, RootSeed256, MTGML_RNG_V1};
use crate::stream_key::RandomStreamKeyV1;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

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
    let mut entries: Vec<CanonicalRandomStreamEntryV1> = map
        .iter()
        .map(|(key, cursor)| CanonicalRandomStreamEntryV1 {
            key: *key,
            next_raw_u64: cursor.next_raw_u64,
        })
        .collect();
    entries.sort_by(|a, b| {
        let a_bytes = a.key.to_canonical_bytes();
        let b_bytes = b.key.to_canonical_bytes();
        a_bytes.cmp(&b_bytes)
    });
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalRandomStreamEntryV1 {
    pub key: RandomStreamKeyV1,
    pub next_raw_u64: u64,
}

impl Serialize for CanonicalRandomStreamEntryV1 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct EntryDto<'a> {
            key: &'a RandomStreamKeyV1,
            next_raw_u64: u64,
        }
        EntryDto {
            key: &self.key,
            next_raw_u64: self.next_raw_u64,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CanonicalRandomStreamEntryV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct EntryDto {
            key: RandomStreamKeyV1,
            next_raw_u64: u64,
        }
        let dto = EntryDto::deserialize(deserializer)?;
        Ok(Self {
            key: dto.key,
            next_raw_u64: dto.next_raw_u64,
        })
    }
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
            streams.insert(
                entry.key,
                RandomStreamCursorV1 {
                    next_raw_u64: entry.next_raw_u64,
                },
            );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream_key::RandomStreamKindV1;

    const ALL_ZERO_SEED: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    #[test]
    fn cursor_defaults_to_zero() {
        let cursor = RandomStreamCursorV1::default();
        assert_eq!(cursor.next_raw_u64, 0);
    }

    #[test]
    fn canonical_entries_sorted_by_key_bytes() {
        let p1 = RandomStreamKeyV1::player_scoped(RandomStreamKindV1::SyntheticM1, 1);
        let p2 = RandomStreamKeyV1::player_scoped(RandomStreamKindV1::SyntheticM1, 2);
        let global = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
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
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
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
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
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
    fn serde_roundtrip_preserves_entries() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let mut state = RandomStateV1::new(seed);
        state
            .add_stream(
                RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
                RandomStreamCursorV1 { next_raw_u64: 42 },
            )
            .unwrap();
        state
            .add_stream(
                RandomStreamKeyV1::player_scoped(RandomStreamKindV1::SyntheticM1, 1),
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
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
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
        let k1 = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
        let k2 = RandomStreamKeyV1::player_scoped(RandomStreamKindV1::SyntheticM1, 1);
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
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
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
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
        assert_eq!(
            state.lookup_stream(&key),
            Err(RandomValidationError::StreamNotFound)
        );
    }

    #[test]
    fn set_cursor_updates_existing() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let mut state = RandomStateV1::new(seed);
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
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
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
        assert_eq!(
            state.set_cursor(&key, RandomStreamCursorV1::default()),
            Err(RandomValidationError::StreamNotFound)
        );
    }
}
