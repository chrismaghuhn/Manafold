//! Per-process adapter session: trusted-key capability, the single live
//! synthetic environment instance, and both routing surfaces over it.
//!
//! No game semantics live here. Construction delegates to the environment
//! backend with the adapter-local configuration identity; a reset replaces
//! the whole environment and invalidates every issued token and route.

use crate::config::synthetic_environment_config;
use crate::tokens::{BoundEndpoint, TokenRegistry};
use mtgml_environment::{
    ControllerError, PlayerEndpointHandle, SyntheticM1EnvironmentBackend,
    TrustedEnvironmentController,
};
use mtgml_model::PlayerId;
use mtgml_random::RootSeed256;
use std::collections::HashMap;

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
    routes: HashMap<PlayerId, PlayerEndpointHandle>,
    tokens: TokenRegistry,
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
        let endpoint = environment
            .controller
            .bind_player(player)
            .map_err(|_| BindError::Rejected)?;
        let binding = BoundEndpoint {
            player,
            endpoint: endpoint.clone(),
        };
        let token = environment
            .tokens
            .insert(binding)
            .map_err(|_| BindError::TokenEntropy)?;
        environment.routes.insert(player, endpoint);
        Ok(token)
    }

    /// The direct-call route: the most recent bound handle for a player.
    pub fn route(&self, player: PlayerId) -> Option<&PlayerEndpointHandle> {
        self.environment.as_ref()?.routes.get(&player)
    }

    pub fn resolve_token(&self, token: &str) -> Option<&BoundEndpoint> {
        self.environment.as_ref()?.tokens.resolve(token)
    }
}
