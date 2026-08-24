//! JSON Lines envelope vocabulary: framing caps, request-shape helpers, the
//! closed error-code set, and deterministic response serialization.
//!
//! Responses carry exactly the protocol fields and never echo routing
//! material; the request id alone correlates. Payload fields are base64
//! strings (standard alphabet, padded) of exact canonical bytes.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use mtgml_wire::{encode_canonical, WireContract};
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::io::{self, BufRead, Read};

pub const PROTOCOL_VERSION: u64 = 1;

pub const MAX_INPUT_LINE_BYTES: usize = 8 * 1024 * 1024;

pub const EXIT_OK: i32 = 0;
pub const EXIT_FATAL: i32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeErrorCode {
    ParseError,
    UnknownCommand,
    InvalidParams,
    UnknownToken,
    OversizedInput,
    InternalError,
    MalformedResponse,
    ServiceUnavailable,
}

impl EnvelopeErrorCode {
    pub fn code(self) -> &'static str {
        match self {
            Self::ParseError => "parse_error",
            Self::UnknownCommand => "unknown_command",
            Self::InvalidParams => "invalid_params",
            Self::UnknownToken => "unknown_token",
            Self::OversizedInput => "oversized_input",
            Self::InternalError => "internal_error",
            Self::MalformedResponse => "malformed_response",
            Self::ServiceUnavailable => "service_unavailable",
        }
    }
}

/// One framed stdin line, capped at [`MAX_INPUT_LINE_BYTES`] bytes
/// including the terminator. An over-cap line is refused whole.
pub enum FramedLine {
    Line(Vec<u8>),
    Oversized,
    Eof,
}

pub fn read_line_capped(input: &mut dyn BufRead) -> io::Result<FramedLine> {
    let mut line = Vec::new();
    let mut limited = input.take(MAX_INPUT_LINE_BYTES as u64 + 1);
    let read = limited.read_until(b'\n', &mut line)?;
    if read == 0 {
        return Ok(FramedLine::Eof);
    }
    if read > MAX_INPUT_LINE_BYTES {
        return Ok(FramedLine::Oversized);
    }
    Ok(FramedLine::Line(line))
}

pub fn extract_id(raw: &Value) -> Option<u64> {
    raw.get("id").and_then(Value::as_u64)
}

pub fn ok_envelope(id: Option<u64>, result: Value) -> String {
    json!({"v": PROTOCOL_VERSION, "id": id, "ok": true, "result": result}).to_string()
}

pub fn error_envelope(id: Option<u64>, code: EnvelopeErrorCode) -> String {
    json!({"v": PROTOCOL_VERSION, "id": id, "ok": false, "error": {"code": code.code()}})
        .to_string()
}

pub fn encode_payload<T>(value: &T) -> Option<String>
where
    T: Serialize + WireContract,
{
    encode_canonical(value)
        .ok()
        .map(|bytes| STANDARD.encode(bytes))
}

pub fn decode_wire_bytes(params: &Map<String, Value>, key: &str) -> Option<Vec<u8>> {
    STANDARD.decode(params.get(key)?.as_str()?).ok()
}
