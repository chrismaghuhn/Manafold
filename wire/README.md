# Canonical wire fixtures

`golden/` contains byte-exact canonical UTF-8 JSON shared by Rust and Python.
Readers reject valid-but-noncanonical JSON, duplicate members, floating-point
numbers, noncanonical unsigned decimal strings, noncanonical Base64, unknown
tagged variants, range violations, and semantic cross-field violations.

`negative/manifest.json` is normative. Every listed fixture must be rejected by
both language implementations. JSON Schema is the structural layer; codec and
semantic validation remain mandatory for invariants JSON Schema cannot express.
