use mtgml_state::EngineStateViolation;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransitionViolation {
    #[error("before state is invalid: {0}")]
    BeforeState(EngineStateViolation),
    #[error("after state is invalid: {0}")]
    AfterState(EngineStateViolation),
    #[error("state delta does not exactly reconstruct next state")]
    DeltaReapplication,
    #[error("a rejected response changed authoritative state, RNG, IDs, or events")]
    RejectedMutation,
    #[error("accepted transition did not advance the revision")]
    RevisionDidNotAdvance,
    #[error("semantic event trace and semantic delta audit differ")]
    EventDeltaMismatch,
    #[error("event IDs are not contiguous or revision-bound")]
    EventIdentity,
    #[error("next decision differs from checkpointed execution state")]
    NextDecisionMismatch,
    #[error("terminal or truncated state still exposes a decision")]
    TerminalDecision,
    #[error("episode status is invalid")]
    EpisodeStatus,
    #[error("zone transition identity, snapshots, or last-known information is invalid")]
    ZoneTransition,
    #[error("object cessation event does not match the semantic cursor")]
    ObjectCessation,
    #[error("object trace does not compose to the final state")]
    ObjectTraceIncomplete,
    #[error("perspective occurrence pairing or lifecycle replay is invalid")]
    OccurrencePairing,
    #[error("life event sequence does not compose to the final state")]
    LifeChange,
    #[error("tap event sequence does not compose to the final state")]
    TapChange,
    #[error("decision event sequence does not compose to the final state")]
    DecisionEvent,
    #[error("an accepted response reused the consumed decision identity")]
    DecisionIdentityReused,
    #[error("randomness event sequence does not match checkpointed stream state")]
    Randomness,
    #[error("public outcome code is empty")]
    PublicOutcome,
}
