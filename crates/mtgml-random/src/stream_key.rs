use crate::seed::RandomValidationError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RandomStreamKindV1 {
    SyntheticM1 = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RandomStreamScopeV1 {
    Global,
    Player(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RandomStreamKeyV1 {
    kind: RandomStreamKindV1,
    scope: RandomStreamScopeV1,
}

impl RandomStreamKeyV1 {
    const VERSION: u8 = 1;

    pub fn global(kind: RandomStreamKindV1) -> Self {
        Self {
            kind,
            scope: RandomStreamScopeV1::Global,
        }
    }

    pub fn player_scoped(kind: RandomStreamKindV1, player_id: u64) -> Self {
        Self {
            kind,
            scope: RandomStreamScopeV1::Player(player_id),
        }
    }

    pub fn kind(&self) -> RandomStreamKindV1 {
        self.kind
    }

    pub fn scope(&self) -> RandomStreamScopeV1 {
        self.scope
    }

    pub fn player(&self) -> Option<u64> {
        match self.scope {
            RandomStreamScopeV1::Global => None,
            RandomStreamScopeV1::Player(p) => Some(p),
        }
    }

    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(4 + 8);
        bytes.push(Self::VERSION);
        bytes.extend_from_slice(&self.kind.to_canonical_u16());
        let (scope_tag, player) = self.scope.to_canonical_tag_and_player();
        bytes.push(scope_tag);
        if let Some(p) = player {
            bytes.extend_from_slice(&p.to_be_bytes());
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
        let scope_tag = bytes[3];
        let kind = RandomStreamKindV1::from_canonical_u16(kind_u16)?;
        let player = match scope_tag {
            0 => {
                if bytes.len() != 4 {
                    return Err(RandomValidationError::MalformedStreamKey);
                }
                None
            }
            1 => {
                if bytes.len() != 12 {
                    return Err(RandomValidationError::MalformedStreamKey);
                }
                let p = u64::from_be_bytes([
                    bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10],
                    bytes[11],
                ]);
                Some(p)
            }
            _ => return Err(RandomValidationError::UnknownScopeTag(scope_tag)),
        };
        let scope = RandomStreamScopeV1::from_canonical_tag(scope_tag, player)?;
        Ok(Self { kind, scope })
    }
}

impl RandomStreamKindV1 {
    fn to_canonical_u16(self) -> [u8; 2] {
        (self as u16).to_be_bytes()
    }

    fn from_canonical_u16(u: u16) -> Result<Self, RandomValidationError> {
        match u {
            1 => Ok(Self::SyntheticM1),
            0 => Err(RandomValidationError::ReservedKind(0)),
            _ => Err(RandomValidationError::UnknownKind(u)),
        }
    }
}

impl RandomStreamScopeV1 {
    fn to_canonical_tag_and_player(self) -> (u8, Option<u64>) {
        match self {
            Self::Global => (0, None),
            Self::Player(p) => (1, Some(p)),
        }
    }

    fn from_canonical_tag(tag: u8, player: Option<u64>) -> Result<Self, RandomValidationError> {
        match tag {
            0 => {
                if player.is_some() {
                    return Err(RandomValidationError::MalformedStreamKey);
                }
                Ok(Self::Global)
            }
            1 => {
                if let Some(p) = player {
                    Ok(Self::Player(p))
                } else {
                    Err(RandomValidationError::MalformedStreamKey)
                }
            }
            _ => Err(RandomValidationError::UnknownScopeTag(tag)),
        }
    }
}

impl Serialize for RandomStreamScopeV1 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let bytes = match self {
            Self::Global => vec![0u8],
            Self::Player(p) => {
                let mut v = vec![1u8];
                v.extend_from_slice(&p.to_be_bytes());
                v
            }
        };
        serializer.serialize_bytes(&bytes)
    }
}

impl<'de> Deserialize<'de> for RandomStreamScopeV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        if bytes.is_empty() {
            return Err(serde::de::Error::custom("empty scope bytes"));
        }
        let tag = bytes[0];
        let player = match tag {
            0 => {
                if bytes.len() != 1 {
                    return Err(serde::de::Error::custom("malformed global scope"));
                }
                None
            }
            1 => {
                if bytes.len() != 9 {
                    return Err(serde::de::Error::custom("malformed player scope"));
                }
                Some(u64::from_be_bytes([
                    bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8],
                ]))
            }
            _ => {
                return Err(serde::de::Error::custom(format!(
                    "unknown scope tag: {}",
                    tag
                )))
            }
        };
        Self::from_canonical_tag(tag, player).map_err(serde::de::Error::custom)
    }
}

impl Serialize for RandomStreamKeyV1 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.to_canonical_bytes())
    }
}

impl<'de> Deserialize<'de> for RandomStreamKeyV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        Self::from_canonical_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_key_global_canonical_roundtrip() {
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
        let bytes = key.to_canonical_bytes();
        let parsed = RandomStreamKeyV1::from_canonical_bytes(&bytes).unwrap();
        assert_eq!(key, parsed);
        assert_eq!(bytes, parsed.to_canonical_bytes());
    }

    #[test]
    fn stream_key_player_canonical_roundtrip() {
        let key = RandomStreamKeyV1::player_scoped(RandomStreamKindV1::SyntheticM1, 42);
        let bytes = key.to_canonical_bytes();
        let parsed = RandomStreamKeyV1::from_canonical_bytes(&bytes).unwrap();
        assert_eq!(key, parsed);
        assert_eq!(bytes, parsed.to_canonical_bytes());
    }

    #[test]
    fn stream_key_rejects_wrong_version() {
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
        let mut bytes = key.to_canonical_bytes();
        bytes[0] = 2;
        assert!(RandomStreamKeyV1::from_canonical_bytes(&bytes).is_err());
    }

    #[test]
    fn stream_key_rejects_unknown_kind() {
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
        let mut bytes = key.to_canonical_bytes();
        bytes[2] = 99;
        assert!(RandomStreamKeyV1::from_canonical_bytes(&bytes).is_err());
    }

    #[test]
    fn stream_key_rejects_unknown_scope() {
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
        let mut bytes = key.to_canonical_bytes();
        bytes[3] = 99;
        assert!(RandomStreamKeyV1::from_canonical_bytes(&bytes).is_err());
    }

    #[test]
    fn stream_key_rejects_malformed_player() {
        let key = RandomStreamKeyV1::player_scoped(RandomStreamKindV1::SyntheticM1, 42);
        let mut bytes = key.to_canonical_bytes();
        bytes.truncate(6);
        assert!(RandomStreamKeyV1::from_canonical_bytes(&bytes).is_err());
    }
}
