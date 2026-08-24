//! Opaque player routing tokens and their binding registry.
//!
//! Tokens are 128-bit values drawn from OS entropy (`getrandom`), issued
//! once by `bind_player` and thereafter inbound-only: no response ever
//! repeats one. Staleness safety comes from replacement, not counters: a
//! reset builds a fresh, empty registry, so previously issued tokens can
//! never resolve again and stale tokens uniformly resolve to nothing.

use getrandom::getrandom;
use mtgml_environment::PlayerEndpointHandle;
use mtgml_model::PlayerId;
use std::collections::HashMap;

const TOKEN_BYTES: usize = 16;

#[derive(Debug)]
pub struct TokenEntropyError;

#[derive(Clone)]
pub struct BoundEndpoint {
    pub player: PlayerId,
    pub endpoint: PlayerEndpointHandle,
}

#[derive(Default)]
pub struct TokenRegistry {
    bindings: HashMap<String, BoundEndpoint>,
}

impl TokenRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, endpoint: BoundEndpoint) -> Result<String, TokenEntropyError> {
        let token = mint_token()?;
        self.bindings.insert(token.clone(), endpoint);
        Ok(token)
    }

    pub fn resolve(&self, token: &str) -> Option<&BoundEndpoint> {
        self.bindings.get(token)
    }
}

fn mint_token() -> Result<String, TokenEntropyError> {
    let mut bytes = [0u8; TOKEN_BYTES];
    getrandom(&mut bytes).map_err(|_| TokenEntropyError)?;
    Ok(encode_lower_hex(&bytes))
}

fn encode_lower_hex(bytes: &[u8]) -> String {
    const HEX: [char; 16] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
    ];
    bytes
        .iter()
        .flat_map(|byte| [HEX[(byte >> 4) as usize], HEX[(byte & 0x0f) as usize]])
        .collect()
}
