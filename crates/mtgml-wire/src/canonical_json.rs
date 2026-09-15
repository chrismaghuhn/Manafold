use crate::contract::WireContract;
use crate::error::WireError;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{Map, Value};

pub fn encode_canonical<T>(value: &T) -> Result<Vec<u8>, WireError>
where
    T: Serialize + WireContract,
{
    value.validate_wire()?;
    let value = serde_json::to_value(value)
        .map_err(|error| WireError::new("encode.serialization", error.to_string()))?;
    serde_json::to_vec(&canonicalize(value))
        .map_err(|error| WireError::new("encode.serialization", error.to_string()))
}

pub fn decode_canonical<T>(bytes: &[u8]) -> Result<T, WireError>
where
    T: DeserializeOwned + Serialize + WireContract,
{
    // Preserved order for every pre-existing wire consumer: closed semantic
    // validation precedes the canonical byte comparison.
    let value: T = serde_json::from_slice(bytes)
        .map_err(|error| WireError::new("decode.invalid_json", error.to_string()))?;
    value.validate_wire()?;
    let canonical = encode_canonical(&value)?;
    if canonical != bytes {
        return Err(WireError::new(
            "decode.non_canonical_json",
            "wire bytes are valid JSON but not the canonical representation",
        ));
    }
    Ok(value)
}

/// Canonical shape decoding only: JSON parse, closed serde shape/types, and
/// canonical byte comparison. Deliberately skips `WireContract` semantic
/// validation so request-relative checks can run later at their owning
/// layer. Not public API; the public submission entry composes this.
pub(crate) fn decode_canonical_shape<T>(bytes: &[u8]) -> Result<T, WireError>
where
    T: DeserializeOwned + Serialize,
{
    let value: T = serde_json::from_slice(bytes)
        .map_err(|error| WireError::new("decode.invalid_json", error.to_string()))?;
    let canonical = encode_shape(&value)?;
    if canonical != bytes {
        return Err(WireError::new(
            "decode.non_canonical_json",
            "wire bytes are valid JSON but not the canonical representation",
        ));
    }
    Ok(value)
}

/// Shape-only canonical encoding (no semantic validation), used exclusively
/// by `decode_canonical_shape` for the byte-equality check.
fn encode_shape<T: Serialize>(value: &T) -> Result<Vec<u8>, WireError> {
    let value = serde_json::to_value(value)
        .map_err(|error| WireError::new("encode.serialization", error.to_string()))?;
    serde_json::to_vec(&canonicalize(value))
        .map_err(|error| WireError::new("encode.serialization", error.to_string()))
}

fn canonicalize(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(canonicalize).collect()),
        Value::Object(object) => {
            let mut pairs: Vec<_> = object.into_iter().collect();
            pairs.sort_by(|left, right| left.0.cmp(&right.0));
            let mut sorted = Map::new();
            for (key, value) in pairs {
                sorted.insert(key, canonicalize(value));
            }
            Value::Object(sorted)
        }
        scalar => scalar,
    }
}
