//! Per-process adapter session: trusted-key capability, the single live
//! synthetic environment instance, and both routing surfaces over it.
//!
//! No game semantics live here. Construction delegates to the environment
//! backend with the adapter-local configuration identity; a reset replaces
//! the whole environment and invalidates every issued token and route.

use crate::config::synthetic_environment_config;
use crate::tokens::{BoundEndpoint, TokenRegistry};
use mtgml_environment::{
    ControllerError, PlayerEndpoint, PlayerEndpointHandle, SyntheticM1EnvironmentBackend,
    TrustedEnvironmentController,
};
use mtgml_model::PlayerId;
use mtgml_random::RootSeed256;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug)]
pub enum BindError {
    NoEnvironment,
    Rejected,
    TokenEntropy,
}

pub struct Session {
    trusted_key: Option<String>,
    environment: Option<LiveEnvironment>,
}

struct LiveEnvironment {
    controller: TrustedEnvironmentController,
    routes: HashMap<PlayerId, Arc<dyn PlayerEndpoint>>,
    tokens: TokenRegistry,
}

/// The single token-minting/route-installation path, shared by production
/// binding ([`Session::bind_player`]) and the test-only registration seam
/// ([`Session::bind_endpoint_for_test`]): a fresh token over `endpoint`
/// plus installation of the direct route for `player`. Both callers riding
/// one code path is what makes the seam's "mirrors production binding
/// exactly" property structural rather than copy-duplicated.
fn mint_token_and_route(
    environment: &mut LiveEnvironment,
    player: PlayerId,
    endpoint: Arc<dyn PlayerEndpoint>,
) -> Result<String, BindError> {
    #[cfg(test)]
    let endpoint = wrap_if_submit_panic_armed(endpoint);
    let binding = BoundEndpoint {
        player,
        endpoint: Arc::clone(&endpoint),
    };
    let token = environment
        .tokens
        .insert(binding)
        .map_err(|_| BindError::TokenEntropy)?;
    environment.routes.insert(player, endpoint);
    Ok(token)
}

impl Session {
    pub fn new(trusted_key: Option<String>) -> Self {
        Self {
            trusted_key,
            environment: None,
        }
    }

    pub fn authorize_trusted(&self, presented: Option<&str>) -> bool {
        match (&self.trusted_key, presented) {
            (Some(expected), Some(actual)) => expected == actual,
            _ => false,
        }
    }

    /// Replaces the live environment wholesale on success: a fresh backend
    /// plus a fresh, empty token registry. Replacement of the registry is
    /// what invalidates every previously issued token; every route over the
    /// old environment is dropped with it.
    pub fn reset_synthetic(
        &mut self,
        players: [PlayerId; 2],
        root_seed: RootSeed256,
    ) -> Result<(), ControllerError> {
        let config = synthetic_environment_config(players);
        let backend = SyntheticM1EnvironmentBackend::new(players, root_seed, config)?;
        let controller = TrustedEnvironmentController::new(backend);
        let tokens = TokenRegistry::new();
        self.environment = Some(LiveEnvironment {
            controller,
            routes: HashMap::new(),
            tokens,
        });
        Ok(())
    }

    pub fn bind_player(&mut self, player: PlayerId) -> Result<String, BindError> {
        let environment = self.environment.as_mut().ok_or(BindError::NoEnvironment)?;
        let handle: PlayerEndpointHandle = environment
            .controller
            .bind_player(player)
            .map_err(|_| BindError::Rejected)?;
        mint_token_and_route(environment, player, Arc::new(handle))
    }

    /// The direct-call route: the most recent bound endpoint for a player.
    pub fn route(&self, player: PlayerId) -> Option<&dyn PlayerEndpoint> {
        self.environment
            .as_ref()?
            .routes
            .get(&player)
            .map(|endpoint| endpoint.as_ref())
    }

    pub fn resolve_token(&self, token: &str) -> Option<&BoundEndpoint> {
        self.environment.as_ref()?.tokens.resolve(token)
    }

    /// Test-only seam (never compiled into production behavior): resolves
    /// an UNREGISTERED controller view of the player's real endpoint
    /// handle, purely for below-envelope comparison in transparency tests.
    #[cfg(test)]
    pub(crate) fn live_handle_for_test(&self, player: PlayerId) -> Option<PlayerEndpointHandle> {
        let environment = self.environment.as_ref()?;
        environment.controller.bind_player(player).ok()
    }

    /// Test-only seam (never compiled into production behavior): registers
    /// a caller-supplied endpoint implementation (e.g. a counting wrapper
    /// or a controlled-failing stub around the real handle) through the
    /// same [`mint_token_and_route`] path as [`Session::bind_player`], so
    /// the observable binding effects — a fresh token plus the direct
    /// route — are structurally identical to production binding.
    #[cfg(test)]
    pub(crate) fn bind_endpoint_for_test(
        &mut self,
        player: PlayerId,
        endpoint: Arc<dyn PlayerEndpoint>,
    ) -> Result<String, BindError> {
        let environment = self.environment.as_mut().ok_or(BindError::NoEnvironment)?;
        mint_token_and_route(environment, player, endpoint)
    }
}

// ---------------------------------------------------------------------------
// Test-only submit-panic injection (run-level failure-proof support)
// ---------------------------------------------------------------------------

#[cfg(test)]
thread_local! {
    static SUBMIT_PANIC_ARMED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Arms (or disarms) the test-only wrapper below: while armed, EVERY
/// endpoint registered through the shared binding path — including plain
/// production `bind_player`, which is how the process loop binds — has its
/// semantic submit replaced by a panic while every other operation keeps
/// forwarding to the real handle. This lets the evidence suite prove the
/// closed panic policy end-to-end through [`crate::run`] without touching
/// any non-test build.
#[cfg(test)]
pub(crate) fn arm_submit_panic(armed: bool) {
    SUBMIT_PANIC_ARMED.with(|cell| cell.set(armed));
}

#[cfg(test)]
fn wrap_if_submit_panic_armed(endpoint: Arc<dyn PlayerEndpoint>) -> Arc<dyn PlayerEndpoint> {
    if SUBMIT_PANIC_ARMED.with(std::cell::Cell::get) {
        Arc::new(PanickingSubmit { inner: endpoint })
    } else {
        endpoint
    }
}

/// Test-only decorator: every operation except `submit` forwards verbatim,
/// so only the semantic submit detonates.
#[cfg(test)]
struct PanickingSubmit {
    inner: Arc<dyn PlayerEndpoint>,
}

#[cfg(test)]
impl PlayerEndpoint for PanickingSubmit {
    fn perspective(&self) -> PlayerId {
        self.inner.perspective()
    }

    fn observation(&self) -> Result<ObservationEnvelope, PlayerEndpointError> {
        self.inner.observation()
    }

    fn information_state(&self) -> Result<PlayerInformationStateV2, PlayerEndpointError> {
        self.inner.information_state()
    }

    fn visible_decision(&self) -> Result<Option<PlayerDecisionRequestV2>, PlayerEndpointError> {
        self.inner.visible_decision()
    }

    fn submit(&self, _response: DecisionResponseV2) -> Result<PlayerStepV2, PlayerEndpointError> {
        panic!("synthetic submit detonation")
    }
}

#[cfg(test)]
use mtgml_decision::{DecisionResponseV2, PlayerDecisionRequestV2};
#[cfg(test)]
use mtgml_environment::PlayerEndpointError;
#[cfg(test)]
use mtgml_observation::{ObservationEnvelope, PlayerInformationStateV2, PlayerStepV2};
