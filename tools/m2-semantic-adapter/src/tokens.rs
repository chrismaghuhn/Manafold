//! Opaque player routing tokens and their binding registry.
//!
//! Tokens are 128-bit values drawn from OS entropy (`getrandom`), issued
//! once by `bind_player` and thereafter inbound-only: no response ever
//! repeats one. Staleness safety comes from replacement, not counters: a
//! reset builds a fresh, empty registry, so previously issued tokens can
//! never resolve again and stale tokens uniformly resolve to nothing.

use getrandom::getrandom;
use mtgml_environment::PlayerEndpoint;
use mtgml_model::PlayerId;
use std::collections::HashMap;
use std::sync::Arc;

const TOKEN_BYTES: usize = 16;

/// Mint-time failure of [`TokenRegistry::insert`]: either OS entropy was
/// unavailable or the freshly minted token collided with an existing
/// binding. Both fail closed — trusted setup may fail loudly; there is
/// deliberately no retry loop.
#[derive(Debug)]
pub struct TokenEntropyError;

/// A bound routing target. The endpoint is held as a shared
/// [`PlayerEndpoint`] trait object so the session can serve either the real
/// controller-bound handle or, under `cfg(test)`, an equivalent test-only
/// wrapper installed around one; the player-facing behavior is identical.
#[derive(Clone)]
pub struct BoundEndpoint {
    pub player: PlayerId,
    pub endpoint: Arc<dyn PlayerEndpoint>,
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
        // Fail closed on the astronomically unlikely 128-bit collision:
        // silently rebinding would hand an already-issued capability to a
        // different endpoint, so the second mint errors and the first
        // binding stays untouched.
        if self.bindings.contains_key(&token) {
            return Err(TokenEntropyError);
        }
        self.bindings.insert(token.clone(), endpoint);
        Ok(token)
    }

    pub fn resolve(&self, token: &str) -> Option<&BoundEndpoint> {
        self.bindings.get(token)
    }
}

#[cfg(test)]
thread_local! {
    static FORCED_MINTED_TOKEN: std::cell::RefCell<Option<String>> =
        const { std::cell::RefCell::new(None) };
}

/// Test-only deterministic seam (never compiled into production behavior):
/// while a token is armed, [`mint_token`] returns it instead of drawing OS
/// entropy, so the collision branch of [`TokenRegistry::insert`] is
/// reachable without relying on an actual 128-bit collision. Mirrors the
/// established `arm_submit_panic` seam in `session.rs`.
#[cfg(test)]
pub(crate) fn force_minted_token(token: Option<String>) {
    FORCED_MINTED_TOKEN.with(|cell| *cell.borrow_mut() = token);
}

fn mint_token() -> Result<String, TokenEntropyError> {
    #[cfg(test)]
    {
        if let Some(forced) = FORCED_MINTED_TOKEN.with(|cell| cell.borrow().clone()) {
            return Ok(forced);
        }
    }
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

#[cfg(test)]
mod tests {
    use mtgml_decision::{DecisionResponseV2, PlayerDecisionRequestV2};
    use mtgml_environment::PlayerEndpointError;
    use mtgml_model::PlayerId;
    use mtgml_observation::{ObservationEnvelope, PlayerInformationStateV2, PlayerStepV2};
    use std::sync::Arc;

    use super::*;

    /// Minimal inert endpoint: the registry only stores and resolves it,
    /// never calls it.
    struct InertEndpoint {
        perspective: PlayerId,
    }

    impl PlayerEndpoint for InertEndpoint {
        fn perspective(&self) -> PlayerId {
            self.perspective
        }

        fn observation(&self) -> Result<ObservationEnvelope, PlayerEndpointError> {
            Err(PlayerEndpointError::ServiceUnavailable)
        }

        fn information_state(&self) -> Result<PlayerInformationStateV2, PlayerEndpointError> {
            Err(PlayerEndpointError::ServiceUnavailable)
        }

        fn visible_decision(&self) -> Result<Option<PlayerDecisionRequestV2>, PlayerEndpointError> {
            Err(PlayerEndpointError::ServiceUnavailable)
        }

        fn submit(
            &self,
            _response: DecisionResponseV2,
        ) -> Result<PlayerStepV2, PlayerEndpointError> {
            Err(PlayerEndpointError::ServiceUnavailable)
        }
    }

    #[test]
    fn colliding_second_insert_fails_closed_and_keeps_the_first_binding() {
        let mut registry = TokenRegistry::new();
        let first_endpoint: Arc<dyn PlayerEndpoint> = Arc::new(InertEndpoint {
            perspective: PlayerId(1),
        });
        let token = registry
            .insert(BoundEndpoint {
                player: PlayerId(1),
                endpoint: Arc::clone(&first_endpoint),
            })
            .expect("first mint succeeds");

        force_minted_token(Some(token.clone()));
        let second = registry.insert(BoundEndpoint {
            player: PlayerId(2),
            endpoint: Arc::new(InertEndpoint {
                perspective: PlayerId(2),
            }),
        });
        assert!(second.is_err(), "colliding second insert must fail closed");
        force_minted_token(None);

        // The first capability is unchanged: the ORIGINAL endpoint still
        // resolves under the original token.
        let resolved = registry.resolve(&token).expect("first binding survives");
        assert_eq!(resolved.player, PlayerId(1));
        assert!(Arc::ptr_eq(&resolved.endpoint, &first_endpoint));
    }
}
