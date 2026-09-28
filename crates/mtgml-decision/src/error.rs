use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DecisionValidationError {
    #[error("unsupported schema version")]
    SchemaVersion,
    #[error("candidate identity fields must be non-empty")]
    EmptyCandidateIdentity,
    #[error("candidate IDs must be unique")]
    DuplicateCandidateId,
    #[error("semantic keys must be unique within a decision")]
    DuplicateSemanticKey,
    #[error("decision bounds are inverted")]
    InvertedBounds,
    #[error("minimum selection cannot be satisfied by the candidate set")]
    ImpossibleMinimum,
    #[error("this decision domain does not accept candidates")]
    CandidatesNotAllowed,
    #[error("decision response contains the same candidate more than once")]
    DuplicateAssignment,
    #[error("answer variant does not match the decision domain")]
    AnswerDomainMismatch,
    #[error("answer contains an unknown candidate")]
    UnknownCandidate,
    #[error("answer contains the same candidate more than once")]
    DuplicateAnswerCandidate,
    #[error("answer is not in its canonical representation")]
    NoncanonicalAnswer,
    #[error("answer cardinality is outside the decision bounds")]
    AnswerCardinality,
    #[error("numeric answer is outside the decision bounds")]
    NumericOutOfBounds,
    #[error("candidate IDs must be dense from zero")]
    CandidateIdsNotDense,
    #[error("candidate ordering is not canonical")]
    NoncanonicalCandidateOrder,
    #[error("candidate ordering contains a duplicate public key")]
    DuplicateOrderingKey,
    #[error("visible candidate and trusted binding variants differ")]
    BindingVariantMismatch,
    #[error("visible cost-route descriptor differs from its trusted route key")]
    CostRouteMismatch,
    #[error("player decision identity does not match the request")]
    DecisionIdentityMismatch,
    #[error("state revision does not match the request")]
    StateRevisionMismatch,
    #[error("candidate value is outside the supported range")]
    ValueOutOfRange,
    #[error("candidate count exceeds the CandidateIdV1 capacity")]
    CandidateCapacityExceeded,
    #[error("decision purpose does not match the answer domain")]
    PurposeDomainMismatch,
    #[error("decision purpose does not match one or more candidate intents")]
    PurposeIntentMismatch,
    #[error("decision visibility is incompatible with the purpose")]
    DecisionVisibilityMismatch,
    #[error("visible candidate domain is not exactly the immutable profile domain")]
    CandidateDomainMismatch,
    #[error("trigger event and safe subject tags differ")]
    TriggerSubjectMismatch,
    #[error("a visible source ability requires its visible source object")]
    SourceAbilityWithoutObject,
    #[error("one opaque ability identity maps to conflicting source objects")]
    ConflictingAbilitySource,
    #[error("response view sequence does not match the request")]
    VisibleSequenceMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CandidateBindingError {
    #[error("visible candidate does not exactly match its authoritative binding")]
    Mismatch,
}
