use crate::PersistenceDecodeErrorV1;

pub const MAX_PAYLOAD_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_TEXT_BYTES: usize = 1024 * 1024;
pub const MAX_BYTE_STRING_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_ARRAY_ELEMENTS: usize = 1024 * 1024;
pub const MAX_DEPTH: usize = 64;
pub const MAX_ITEMS: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    Unsigned(u64),
    Signed(i64),
    Bytes(Vec<u8>),
    Text(String),
    Array(Vec<Value>),
}

pub fn encode_canonical(value: &Value) -> Result<Vec<u8>, PersistenceDecodeErrorV1> {
    let mut encoder = Encoder {
        output: Vec::new(),
        items: 0,
    };
    encoder.write_value(value, 0)?;
    if encoder.output.len() > MAX_PAYLOAD_BYTES {
        return Err(PersistenceDecodeErrorV1::PayloadTooLarge);
    }
    Ok(encoder.output)
}

pub fn decode_canonical(bytes: &[u8]) -> Result<Value, PersistenceDecodeErrorV1> {
    if bytes.len() > MAX_PAYLOAD_BYTES {
        return Err(PersistenceDecodeErrorV1::PayloadTooLarge);
    }
    let mut decoder = Decoder {
        input: bytes,
        offset: 0,
        items: 0,
    };
    let value = decoder.read_value(0)?;
    if decoder.offset != bytes.len() {
        return Err(PersistenceDecodeErrorV1::TrailingData);
    }
    let reencoded = encode_canonical(&value)?;
    if reencoded != bytes {
        return Err(PersistenceDecodeErrorV1::ReencodeMismatch);
    }
    Ok(value)
}

struct Encoder {
    output: Vec<u8>,
    items: usize,
}

impl Encoder {
    fn write_value(&mut self, value: &Value, depth: usize) -> Result<(), PersistenceDecodeErrorV1> {
        self.items = self
            .items
            .checked_add(1)
            .ok_or(PersistenceDecodeErrorV1::ItemLimitExceeded)?;
        if self.items > MAX_ITEMS {
            return Err(PersistenceDecodeErrorV1::ItemLimitExceeded);
        }
        match value {
            Value::Null => self.output.push(0xf6),
            Value::Bool(false) => self.output.push(0xf4),
            Value::Bool(true) => self.output.push(0xf5),
            Value::Unsigned(number) => write_head(&mut self.output, 0, *number),
            Value::Signed(number) if *number >= 0 => {
                write_head(&mut self.output, 0, *number as u64)
            }
            Value::Signed(number) => {
                let magnitude = (-1i128 - i128::from(*number)) as u64;
                write_head(&mut self.output, 1, magnitude);
            }
            Value::Bytes(bytes) => {
                if bytes.len() > MAX_BYTE_STRING_BYTES {
                    return Err(PersistenceDecodeErrorV1::PayloadTooLarge);
                }
                write_head(&mut self.output, 2, bytes.len() as u64);
                self.output.extend_from_slice(bytes);
            }
            Value::Text(text) => {
                if text.len() > MAX_TEXT_BYTES {
                    return Err(PersistenceDecodeErrorV1::StringTooLarge);
                }
                write_head(&mut self.output, 3, text.len() as u64);
                self.output.extend_from_slice(text.as_bytes());
            }
            Value::Array(values) => {
                if values.len() > MAX_ARRAY_ELEMENTS {
                    return Err(PersistenceDecodeErrorV1::ArrayTooLarge);
                }
                if depth >= MAX_DEPTH {
                    return Err(PersistenceDecodeErrorV1::DepthExceeded);
                }
                write_head(&mut self.output, 4, values.len() as u64);
                for value in values {
                    self.write_value(value, depth + 1)?;
                }
            }
        }
        Ok(())
    }
}

fn write_head(output: &mut Vec<u8>, major: u8, value: u64) {
    let major = major << 5;
    if value <= 23 {
        output.push(major | value as u8);
    } else if value <= u8::MAX as u64 {
        output.push(major | 24);
        output.push(value as u8);
    } else if value <= u16::MAX as u64 {
        output.push(major | 25);
        output.extend_from_slice(&(value as u16).to_be_bytes());
    } else if value <= u32::MAX as u64 {
        output.push(major | 26);
        output.extend_from_slice(&(value as u32).to_be_bytes());
    } else {
        output.push(major | 27);
        output.extend_from_slice(&value.to_be_bytes());
    }
}

struct Decoder<'a> {
    input: &'a [u8],
    offset: usize,
    items: usize,
}

impl<'a> Decoder<'a> {
    fn read_value(&mut self, depth: usize) -> Result<Value, PersistenceDecodeErrorV1> {
        self.items = self
            .items
            .checked_add(1)
            .ok_or(PersistenceDecodeErrorV1::ItemLimitExceeded)?;
        if self.items > MAX_ITEMS {
            return Err(PersistenceDecodeErrorV1::ItemLimitExceeded);
        }
        let (major, argument) = self.read_head()?;
        match major {
            0 => Ok(Value::Unsigned(argument)),
            1 => {
                if argument > i64::MAX as u64 {
                    return Err(PersistenceDecodeErrorV1::ValueOutOfRange);
                }
                Ok(Value::Signed(-1 - argument as i64))
            }
            2 => {
                // ADR-0040: truncation (rank 3) precedes an over-limit
                // declaration (rank 4) for length-prefixed values.
                let declared = usize::try_from(argument)
                    .map_err(|_| PersistenceDecodeErrorV1::EnvelopeLength)?;
                let end = self
                    .offset
                    .checked_add(declared)
                    .ok_or(PersistenceDecodeErrorV1::EnvelopeLength)?;
                if end > self.input.len() {
                    return Err(PersistenceDecodeErrorV1::EnvelopeLength);
                }
                if declared > MAX_BYTE_STRING_BYTES {
                    return Err(PersistenceDecodeErrorV1::PayloadTooLarge);
                }
                Ok(Value::Bytes(self.read_exact(declared)?.to_vec()))
            }
            3 => {
                let declared = usize::try_from(argument)
                    .map_err(|_| PersistenceDecodeErrorV1::EnvelopeLength)?;
                let end = self
                    .offset
                    .checked_add(declared)
                    .ok_or(PersistenceDecodeErrorV1::EnvelopeLength)?;
                if end > self.input.len() {
                    return Err(PersistenceDecodeErrorV1::EnvelopeLength);
                }
                if declared > MAX_TEXT_BYTES {
                    return Err(PersistenceDecodeErrorV1::StringTooLarge);
                }
                let bytes = self.read_exact(declared)?;
                let text = String::from_utf8(bytes.to_vec())
                    .map_err(|_| PersistenceDecodeErrorV1::InvalidUtf8)?;
                Ok(Value::Text(text))
            }
            4 => {
                let length = checked_length(
                    argument,
                    MAX_ARRAY_ELEMENTS,
                    PersistenceDecodeErrorV1::ArrayTooLarge,
                )?;
                if depth >= MAX_DEPTH {
                    return Err(PersistenceDecodeErrorV1::DepthExceeded);
                }
                if self.items.checked_add(length).unwrap_or(usize::MAX) > MAX_ITEMS {
                    return Err(PersistenceDecodeErrorV1::ItemLimitExceeded);
                }
                let mut values = Vec::with_capacity(length);
                for _ in 0..length {
                    values.push(self.read_value(depth + 1)?);
                }
                Ok(Value::Array(values))
            }
            5 | 6 => Err(PersistenceDecodeErrorV1::DisallowedCborForm),
            7 => match argument {
                20 => Ok(Value::Bool(false)),
                21 => Ok(Value::Bool(true)),
                22 => Ok(Value::Null),
                _ => Err(PersistenceDecodeErrorV1::DisallowedCborForm),
            },
            _ => Err(PersistenceDecodeErrorV1::DisallowedCborForm),
        }
    }

    fn read_head(&mut self) -> Result<(u8, u64), PersistenceDecodeErrorV1> {
        let first = *self
            .input
            .get(self.offset)
            .ok_or(PersistenceDecodeErrorV1::EnvelopeLength)?;
        self.offset += 1;
        let major = first >> 5;
        let additional = first & 0x1f;
        // ADR-0040 precedence: a disallowed form is observable from the head
        // itself and precedes any canonical-primitive defect of the argument.
        if matches!(major, 5 | 6) {
            return Err(PersistenceDecodeErrorV1::DisallowedCborForm);
        }
        if additional >= 28 {
            return Err(PersistenceDecodeErrorV1::DisallowedCborForm);
        }
        if major == 7 && additional > 23 {
            // Only false/true/null are allowed; every wider major-7 encoding
            // is a float or an unassigned simple value.
            return Err(PersistenceDecodeErrorV1::DisallowedCborForm);
        }
        let argument = match additional {
            0..=23 => additional as u64,
            24 => {
                let value = self.read_uint_bytes(1)?;
                if value < 24 {
                    return Err(PersistenceDecodeErrorV1::NoncanonicalPrimitive);
                }
                value
            }
            25 => {
                let value = self.read_uint_bytes(2)?;
                if value <= u8::MAX as u64 {
                    return Err(PersistenceDecodeErrorV1::NoncanonicalPrimitive);
                }
                value
            }
            26 => {
                let value = self.read_uint_bytes(4)?;
                if value <= u16::MAX as u64 {
                    return Err(PersistenceDecodeErrorV1::NoncanonicalPrimitive);
                }
                value
            }
            27 => {
                let value = self.read_uint_bytes(8)?;
                if value <= u32::MAX as u64 {
                    return Err(PersistenceDecodeErrorV1::NoncanonicalPrimitive);
                }
                value
            }
            _ => return Err(PersistenceDecodeErrorV1::DisallowedCborForm),
        };
        Ok((major, argument))
    }

    fn read_uint_bytes(&mut self, width: usize) -> Result<u64, PersistenceDecodeErrorV1> {
        let bytes = self.read_exact(width)?;
        let mut value = 0u64;
        for byte in bytes {
            value = (value << 8) | u64::from(*byte);
        }
        Ok(value)
    }

    fn read_exact(&mut self, length: usize) -> Result<&'a [u8], PersistenceDecodeErrorV1> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(PersistenceDecodeErrorV1::EnvelopeLength)?;
        if end > self.input.len() {
            return Err(PersistenceDecodeErrorV1::EnvelopeLength);
        }
        let bytes = &self.input[self.offset..end];
        self.offset = end;
        Ok(bytes)
    }
}

fn checked_length(
    length: u64,
    maximum: usize,
    error: PersistenceDecodeErrorV1,
) -> Result<usize, PersistenceDecodeErrorV1> {
    let length = usize::try_from(length).map_err(|_| error)?;
    if length > maximum {
        return Err(error);
    }
    Ok(length)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope;

    fn bytes_value(length: usize) -> Vec<u8> {
        let length = u32::try_from(length).unwrap();
        let mut bytes = vec![0x5a];
        bytes.extend_from_slice(&length.to_be_bytes());
        bytes.extend(std::iter::repeat_n(0u8, length as usize));
        bytes
    }

    fn text_value(length: usize) -> Vec<u8> {
        let length = u32::try_from(length).unwrap();
        let mut bytes = vec![0x7a];
        bytes.extend_from_slice(&length.to_be_bytes());
        bytes.extend(std::iter::repeat_n(b'a', length as usize));
        bytes
    }

    fn array_of_nulls(length: usize) -> Vec<u8> {
        let length = u32::try_from(length).unwrap();
        let mut bytes = vec![0x9a];
        bytes.extend_from_slice(&length.to_be_bytes());
        bytes.extend(std::iter::repeat_n(0xf6, length as usize));
        bytes
    }

    fn nested_arrays(depth: usize) -> Vec<u8> {
        std::iter::repeat_n(0x81u8, depth)
            .chain(std::iter::once(0x00))
            .collect()
    }

    fn item_boundary_payload(last_child_len: usize) -> Vec<u8> {
        let child_lengths = [1_048_576usize, 1_048_576, 1_048_576, last_child_len];
        let mut bytes = vec![0x84];
        for length in child_lengths {
            bytes.extend_from_slice(&[0x9a]);
            bytes.extend_from_slice(&u32::try_from(length).unwrap().to_be_bytes());
            bytes.extend(std::iter::repeat_n(0xf6, length));
        }
        bytes
    }

    #[test]
    fn cbor_resource_boundaries_are_exact_at_each_declared_boundary() {
        for total in [MAX_PAYLOAD_BYTES - 1, MAX_PAYLOAD_BYTES] {
            assert!(decode_canonical(&bytes_value(total - 5)).is_ok());
        }
        assert_eq!(
            decode_canonical(&vec![0u8; MAX_PAYLOAD_BYTES + 1]).unwrap_err(),
            PersistenceDecodeErrorV1::PayloadTooLarge
        );

        assert!(decode_canonical(&text_value(MAX_TEXT_BYTES - 1)).is_ok());
        assert!(decode_canonical(&text_value(MAX_TEXT_BYTES)).is_ok());
        assert_eq!(
            decode_canonical(&text_value(MAX_TEXT_BYTES + 1)).unwrap_err(),
            PersistenceDecodeErrorV1::StringTooLarge
        );

        assert!(decode_canonical(&array_of_nulls(MAX_ARRAY_ELEMENTS - 1)).is_ok());
        assert!(decode_canonical(&array_of_nulls(MAX_ARRAY_ELEMENTS)).is_ok());
        assert_eq!(
            decode_canonical(&array_of_nulls(MAX_ARRAY_ELEMENTS + 1)).unwrap_err(),
            PersistenceDecodeErrorV1::ArrayTooLarge
        );

        assert!(decode_canonical(&nested_arrays(MAX_DEPTH - 1)).is_ok());
        assert!(decode_canonical(&nested_arrays(MAX_DEPTH)).is_ok());
        assert_eq!(
            decode_canonical(&nested_arrays(MAX_DEPTH + 1)).unwrap_err(),
            PersistenceDecodeErrorV1::DepthExceeded
        );

        assert!(decode_canonical(&item_boundary_payload(1_048_570)).is_ok());
        assert!(decode_canonical(&item_boundary_payload(1_048_571)).is_ok());
        assert_eq!(
            decode_canonical(&item_boundary_payload(1_048_572)).unwrap_err(),
            PersistenceDecodeErrorV1::ItemLimitExceeded
        );

        for length in [MAX_BYTE_STRING_BYTES - 1, MAX_BYTE_STRING_BYTES] {
            let bytes = bytes_value(length);
            let mut decoder = Decoder {
                input: &bytes,
                offset: 0,
                items: 0,
            };
            assert!(decoder.read_value(0).is_ok());
        }
        let bytes = bytes_value(MAX_BYTE_STRING_BYTES + 1);
        let mut decoder = Decoder {
            input: &bytes,
            offset: 0,
            items: 0,
        };
        assert_eq!(
            decoder.read_value(0).unwrap_err(),
            PersistenceDecodeErrorV1::PayloadTooLarge
        );

        let payload = encode_canonical(&Value::Array(vec![Value::Unsigned(0)])).unwrap();
        for length in [
            envelope::MAX_IDENTIFIER_BYTES - 1,
            envelope::MAX_IDENTIFIER_BYTES,
        ] {
            let identifier = "a".repeat(length);
            assert!(envelope::encode_envelope(&identifier, "schema", &payload).is_ok());
        }
        assert_eq!(
            envelope::encode_envelope(
                &"a".repeat(envelope::MAX_IDENTIFIER_BYTES + 1),
                "schema",
                &payload
            )
            .unwrap_err(),
            PersistenceDecodeErrorV1::EnvelopeIdentity
        );
    }
}
