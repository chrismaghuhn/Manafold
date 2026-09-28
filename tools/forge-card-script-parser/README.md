# Forge card script parser

This standalone utility parses Forge's line-oriented card script syntax into
ordinary Rust structs. It preserves fields in source order, including repeated
and unknown fields, and keeps each raw value. It does not compile or interpret
card behavior.

Run the example CLI with:

```sh
cargo run \
  --manifest-path tools/forge-card-script-parser/Cargo.toml \
  --bin forge-card-script \
  -- path/to/card-script.txt
```

Library callers can use `forge_card_script_parser::parse_card_script(source)`.
For ability fields (`A`, `T`, `R`, `S`), a best-effort syntax view is available
alongside the unmodified raw value.
