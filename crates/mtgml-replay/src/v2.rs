//! The randomness identity that Replay V8 embeds.

use mtgml_random::types::validate_seed_hex;
use serde::{Deserialize, Deserializer, Serialize};

fn deserialize_root_seed_hex<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    validate_seed_hex(&s).map_err(|_| {
        serde::de::Error::custom("root seed is not canonical lowercase hexadecimal")
    })?;
    Ok(s)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RandomnessIdentityV2 {
    pub contract_id: String,
    #[serde(deserialize_with = "deserialize_root_seed_hex")]
    pub root_seed_hex: String,
}
