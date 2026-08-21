use thiserror::Error;

/// Closed, versioned categories returned before persisted data becomes a
/// trusted runtime value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PersistenceDecodeErrorV1 {
    #[error("unsupported historical version")]
    UnsupportedHistoricalVersion,
    #[error("digest envelope identity is invalid")]
    EnvelopeIdentity,
    #[error("digest envelope length is invalid")]
    EnvelopeLength,
    #[error("canonical payload exceeds the configured limit")]
    PayloadTooLarge,
    #[error("text string exceeds the configured limit")]
    StringTooLarge,
    #[error("array exceeds the configured limit")]
    ArrayTooLarge,
    #[error("canonical CBOR nesting exceeds the configured limit")]
    DepthExceeded,
    #[error("canonical CBOR item count exceeds the configured limit")]
    ItemLimitExceeded,
    #[error("CBOR form is not allowed by the canonical profile")]
    DisallowedCborForm,
    #[error("CBOR primitive is not in its canonical representation")]
    NoncanonicalPrimitive,
    #[error("CBOR text is not valid UTF-8")]
    InvalidUtf8,
    #[error("record has the wrong number of fields")]
    WrongRecordLength,
    #[error("record variant is unknown")]
    UnknownVariant,
    #[error("value is outside its declared range")]
    ValueOutOfRange,
    #[error("semantic keys contain a duplicate")]
    DuplicateSemanticKey,
    #[error("semantic entries are not in canonical order")]
    NoncanonicalOrder,
    #[error("schema identity does not match the declared contract")]
    SchemaIdentityMismatch,
    #[error("trailing bytes follow the canonical value")]
    TrailingData,
    #[error("decoded value does not re-encode canonically")]
    ReencodeMismatch,
    #[error("digest does not match the envelope contents")]
    DigestMismatch,
    #[error("semantic validation failed")]
    SemanticValidation,
}

impl PersistenceDecodeErrorV1 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedHistoricalVersion => "unsupported_historical_version",
            Self::EnvelopeIdentity => "envelope_identity",
            Self::EnvelopeLength => "envelope_length",
            Self::PayloadTooLarge => "payload_too_large",
            Self::StringTooLarge => "string_too_large",
            Self::ArrayTooLarge => "array_too_large",
            Self::DepthExceeded => "depth_exceeded",
            Self::ItemLimitExceeded => "item_limit_exceeded",
            Self::DisallowedCborForm => "disallowed_cbor_form",
            Self::NoncanonicalPrimitive => "noncanonical_primitive",
            Self::InvalidUtf8 => "invalid_utf8",
            Self::WrongRecordLength => "wrong_record_length",
            Self::UnknownVariant => "unknown_variant",
            Self::ValueOutOfRange => "value_out_of_range",
            Self::DuplicateSemanticKey => "duplicate_semantic_key",
            Self::NoncanonicalOrder => "noncanonical_order",
            Self::SchemaIdentityMismatch => "schema_identity_mismatch",
            Self::TrailingData => "trailing_data",
            Self::ReencodeMismatch => "reencode_mismatch",
            Self::DigestMismatch => "digest_mismatch",
            Self::SemanticValidation => "semantic_validation",
        }
    }
}

/// The category-only view used by fixture/reporting code.
pub type PersistenceErrorCategory = PersistenceDecodeErrorV1;
