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
    #[error("player decision identity does not match the request")]
    DecisionIdentityMismatch,
    #[error("state revision does not match the request")]
    StateRevisionMismatch,
    #[error("candidate value is outside the supported range")]
    ValueOutOfRange,
    #[error("candidate count exceeds the CandidateIdV1 capacity")]
    CandidateCapacityExceeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CandidateBindingError {
    #[error("visible candidate does not exactly match its authoritative binding")]
    Mismatch,
}
