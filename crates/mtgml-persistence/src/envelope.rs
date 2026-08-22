use crate::{cbor, PersistenceDecodeErrorV1};
use mtgml_model::DigestReferenceV1;
use sha2::{Digest as _, Sha256};

pub const DIGEST_ENVELOPE_ID: &str = "mtgml.digest-envelope.v1";
pub const SHA256_ID: &str = "sha-256";
pub const CANONICAL_CBOR_ID: &str = "mtgml.canonical-cbor.v1";
pub const MAX_IDENTIFIER_BYTES: usize = 255;

pub fn encode_envelope(
    semantic_domain: &str,
    input_schema_id: &str,
    canonical_payload: &[u8],
) -> Result<Vec<u8>, PersistenceDecodeErrorV1> {
    validate_identifier(semantic_domain)?;
    validate_identifier(input_schema_id)?;
    if canonical_payload.len() > cbor::MAX_PAYLOAD_BYTES {
        return Err(PersistenceDecodeErrorV1::PayloadTooLarge);
    }
    cbor::decode_canonical(canonical_payload)?;

    let mut output = Vec::with_capacity(
        DIGEST_ENVELOPE_ID.len()
            + 1
            + (8 * 5)
            + SHA256_ID.len()
            + semantic_domain.len()
            + CANONICAL_CBOR_ID.len()
            + input_schema_id.len()
            + canonical_payload.len(),
    );
    output.extend_from_slice(DIGEST_ENVELOPE_ID.as_bytes());
    output.push(0);
    for field in [
        SHA256_ID.as_bytes(),
        semantic_domain.as_bytes(),
        CANONICAL_CBOR_ID.as_bytes(),
        input_schema_id.as_bytes(),
        canonical_payload,
    ] {
        write_frame(&mut output, field)?;
    }
    Ok(output)
}

pub fn hash_envelope(envelope: &[u8]) -> [u8; 32] {
    Sha256::digest(envelope).into()
}

pub fn decode_envelope(
    envelope: &[u8],
) -> Result<(DigestReferenceV1, Vec<u8>), PersistenceDecodeErrorV1> {
    // ADR-0040 total precedence. Every identity field is validated
    // immediately after its own frame is read, so an identity defect always
    // precedes any later framing or payload defect (rank 2 < 3 < 4).
    let prefix = [DIGEST_ENVELOPE_ID.as_bytes(), &[0]].concat();
    if !envelope.starts_with(&prefix) {
        return Err(PersistenceDecodeErrorV1::EnvelopeIdentity);
    }
    let mut offset = prefix.len();

    let algorithm = read_identifier_frame(envelope, &mut offset)?;
    let algorithm =
        String::from_utf8(algorithm).map_err(|_| PersistenceDecodeErrorV1::EnvelopeIdentity)?;
    validate_identifier(&algorithm)?;
    if algorithm != SHA256_ID {
        return Err(PersistenceDecodeErrorV1::EnvelopeIdentity);
    }

    let semantic_domain = read_identifier_frame(envelope, &mut offset)?;
    let semantic_domain = String::from_utf8(semantic_domain)
        .map_err(|_| PersistenceDecodeErrorV1::EnvelopeIdentity)?;
    validate_identifier(&semantic_domain)?;

    let codec = read_identifier_frame(envelope, &mut offset)?;
    let codec = String::from_utf8(codec).map_err(|_| PersistenceDecodeErrorV1::EnvelopeIdentity)?;
    validate_identifier(&codec)?;
    if codec != CANONICAL_CBOR_ID {
        return Err(PersistenceDecodeErrorV1::EnvelopeIdentity);
    }

    let input_schema = read_identifier_frame(envelope, &mut offset)?;
    let input_schema =
        String::from_utf8(input_schema).map_err(|_| PersistenceDecodeErrorV1::EnvelopeIdentity)?;
    validate_identifier(&input_schema)?;

    // Identity fields are fully validated; only now may payload framing
    // (rank 3) and payload bounds (rank 4) be reported.
    let payload = read_payload_frame(envelope, &mut offset)?;
    if offset != envelope.len() {
        return Err(PersistenceDecodeErrorV1::EnvelopeLength);
    }
    cbor::decode_canonical(&payload)?;
    let digest_bytes = hash_envelope(envelope);
    Ok((
        DigestReferenceV1 {
            envelope_version: DIGEST_ENVELOPE_ID.to_owned(),
            algorithm_id: algorithm,
            semantic_domain,
            payload_codec_id: codec,
            input_schema_id: input_schema,
            digest_bytes,
        },
        payload,
    ))
}

pub fn digest_reference_value(reference: &DigestReferenceV1) -> cbor::Value {
    cbor::Value::Array(vec![
        cbor::Value::Text(reference.envelope_version.clone()),
        cbor::Value::Text(reference.algorithm_id.clone()),
        cbor::Value::Text(reference.semantic_domain.clone()),
        cbor::Value::Text(reference.payload_codec_id.clone()),
        cbor::Value::Text(reference.input_schema_id.clone()),
        cbor::Value::Bytes(reference.digest_bytes.to_vec()),
    ])
}

fn validate_identifier(value: &str) -> Result<(), PersistenceDecodeErrorV1> {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || !value.bytes().all(|byte| byte.is_ascii())
    {
        return Err(PersistenceDecodeErrorV1::EnvelopeIdentity);
    }
    Ok(())
}

fn write_frame(output: &mut Vec<u8>, value: &[u8]) -> Result<(), PersistenceDecodeErrorV1> {
    let length =
        u64::try_from(value.len()).map_err(|_| PersistenceDecodeErrorV1::EnvelopeLength)?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(value);
    Ok(())
}

/// Reads one identifier frame. Precedence for identifiers is rank 2 over
/// rank 3: a declared length above the identifier bound is an identity
/// defect even when the declared bytes are also absent.
fn read_identifier_frame(
    input: &[u8],
    offset: &mut usize,
) -> Result<Vec<u8>, PersistenceDecodeErrorV1> {
    let end_of_length = offset
        .checked_add(8)
        .ok_or(PersistenceDecodeErrorV1::EnvelopeLength)?;
    if end_of_length > input.len() {
        return Err(PersistenceDecodeErrorV1::EnvelopeLength);
    }
    let declared = u64::from_be_bytes(
        input[*offset..end_of_length]
            .try_into()
            .expect("length is 8 bytes"),
    );
    *offset = end_of_length;
    let declared =
        usize::try_from(declared).map_err(|_| PersistenceDecodeErrorV1::EnvelopeIdentity)?;
    if declared > MAX_IDENTIFIER_BYTES {
        return Err(PersistenceDecodeErrorV1::EnvelopeIdentity);
    }
    read_declared_bytes(input, offset, declared)
}

/// Reads the payload frame. Precedence follows ADR-0040: truncation
/// (rank 3) is reported before an over-limit declaration (rank 4).
fn read_payload_frame(
    input: &[u8],
    offset: &mut usize,
) -> Result<Vec<u8>, PersistenceDecodeErrorV1> {
    let end_of_length = offset
        .checked_add(8)
        .ok_or(PersistenceDecodeErrorV1::EnvelopeLength)?;
    if end_of_length > input.len() {
        return Err(PersistenceDecodeErrorV1::EnvelopeLength);
    }
    let declared = u64::from_be_bytes(
        input[*offset..end_of_length]
            .try_into()
            .expect("length is 8 bytes"),
    );
    *offset = end_of_length;
    let declared =
        usize::try_from(declared).map_err(|_| PersistenceDecodeErrorV1::EnvelopeLength)?;
    let remaining = input.len() - *offset;
    if declared > remaining {
        return Err(PersistenceDecodeErrorV1::EnvelopeLength);
    }
    if declared > cbor::MAX_PAYLOAD_BYTES {
        return Err(PersistenceDecodeErrorV1::PayloadTooLarge);
    }
    read_declared_bytes(input, offset, declared)
}

fn read_declared_bytes(
    input: &[u8],
    offset: &mut usize,
    declared: usize,
) -> Result<Vec<u8>, PersistenceDecodeErrorV1> {
    let end = offset
        .checked_add(declared)
        .ok_or(PersistenceDecodeErrorV1::EnvelopeLength)?;
    if end > input.len() {
        return Err(PersistenceDecodeErrorV1::EnvelopeLength);
    }
    let value = input[*offset..end].to_vec();
    *offset = end;
    Ok(value)
}
