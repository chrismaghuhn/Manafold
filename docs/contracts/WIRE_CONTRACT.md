# Wire Contract

**Status:** provisional public wire contract  
**Stability:** normative


The public wire format is canonical compact UTF-8 JSON with lexicographically sorted object keys, no insignificant whitespace, canonical decimal identifier strings, canonical lowercase SHA-256 hex, and canonical padded standard Base64 for byte payloads.

Validation has three layers:

1. JSON Schema: closed shape and obvious scalar bounds.
2. Decoder/encoder: closed variants, canonical scalar encoding, language ranges, and canonical bytes.
3. Semantic validation: cross-field invariants such as `minimum <= maximum`, unique candidate IDs, replay revision continuity, and player-step perspective consistency.

Rust and Python use the fixtures under `wire/`. A contract change is not mergeable unless both readers and writers, schemas, positive fixtures, negative fixtures, and compatibility notes change together.

The encoder is fallible. Invalid domain objects are never serialized as plausible wire data.
