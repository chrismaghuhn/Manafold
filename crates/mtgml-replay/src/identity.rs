use mtgml_model::{ContentDigest, PlayerId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelIdentityV1 {
    pub implementation_id: String,
    pub semantic_version: String,
    pub build_profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeckIdentityV1 {
    pub player: PlayerId,
    pub deck_id: String,
    pub digest: ContentDigest,
}
