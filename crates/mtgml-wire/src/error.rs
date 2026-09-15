use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireError {
    pub code: &'static str,
    pub message: String,
}

impl WireError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for WireError {}

/// Versioned public wire code family for player-boundary decode failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerWireErrorCodeV1 {
    MalformedResponse,
}

impl PlayerWireErrorCodeV1 {
    pub fn code(self) -> &'static str {
        match self {
            Self::MalformedResponse => "malformed_response",
        }
    }
}
