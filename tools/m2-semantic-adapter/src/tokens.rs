//! Opaque player routing tokens and the generation-epoch registry.
//!
//! Tokens are 128-bit values drawn from OS entropy (`getrandom`), issued
//! once by `bind_player` and thereafter inbound-only: no response ever
//! repeats one. A reset bumps the epoch and drops every binding, so stale
//! tokens resolve uniformly to nothing.

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
    epoch: u64,
    bindings: HashMap<String, BoundEndpoint>,
}

impl TokenRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Bumps the generation epoch and drops every binding; previously
    /// issued tokens can never resolve again.
    pub fn invalidate_all(&mut self) -> u64 {
        self.epoch += 1;
        self.bindings.clear();
        self.epoch
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
