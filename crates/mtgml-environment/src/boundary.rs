//! Player wire-boundary adapter: composes the canonical response decoder
//! with the typed player endpoint.
//!
//! This is the accepted Layer-A/Layer-B seam: malformed, noncanonical,
//! unknown-schema, or otherwise shape-invalid bytes fail here with the
//! closed `PlayerWireErrorCodeV1::MalformedResponse` code and never reach
//! [`PlayerEndpoint::submit`]. No transport decision is made; M2.H/M5 own
//! that later.

use mtgml_observation::{PlayerServiceErrorCodeV1, PlayerStepV2};
use mtgml_wire::PlayerWireErrorCodeV1;

use crate::endpoint::{PlayerEndpoint, PlayerEndpointError};

/// Closed public failure of the player wire boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerBoundaryError {
    /// Layer A: bytes failed canonical/shape/schema decoding.
    Wire(PlayerWireErrorCodeV1),
    /// Layer C: closed service failure with no trusted detail. The
    /// versioned code is the single string authority.
    Service(PlayerServiceErrorCodeV1),
}

impl From<PlayerEndpointError> for PlayerBoundaryError {
    fn from(_: PlayerEndpointError) -> Self {
        Self::Service(PlayerServiceErrorCodeV1::ServiceUnavailable)
    }
}

impl PlayerBoundaryError {
    pub fn code(self) -> &'static str {
        match self {
            Self::Wire(code) => code.code(),
            Self::Service(code) => code.code(),
        }
    }
}

/// Canonical-bytes submission entry: decodes the typed response at the wire
/// boundary and only then invokes the semantic endpoint.
pub fn submit_response_bytes(
    endpoint: &dyn PlayerEndpoint,
    bytes: &[u8],
) -> Result<PlayerStepV2, PlayerBoundaryError> {
    let response = mtgml_wire::decision_response_v2::decode_submission(bytes)
        .map_err(|_| PlayerBoundaryError::Wire(PlayerWireErrorCodeV1::MalformedResponse))?;
    Ok(endpoint.submit(response)?)
}
