# Forge card script parser

This standalone utility parses Forge's line-oriented card script syntax into
ordinary Rust structs. It preserves fields in source order, including repeated
and unknown fields, and keeps each raw value. It does not compile or interpret
card behavior. Forge's standalone `ALTERNATE` face separator is retained as an
ordered marker so both sides of a double-faced card file can be parsed.

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

## Lower the selected R1/W1 characteristics

The lowerer handles only the 26 selected cards' printed `Name`, `ManaCost`,
`Types`, and `PT` fields. It verifies the Forge checkout revision and that the
selected source files are unchanged. It does not download Forge or evaluate
Forge scripts.

```sh
cargo run \
  --manifest-path tools/forge-card-script-parser/Cargo.toml \
  --bin forge-card-lower \
  -- /path/to/forge-checkout
```

The default output directory is `tools/forge-card-script-parser/candidates/r1w1/`.
It contains the existing canonical Card IR content manifest and provenance
catalog formats, plus tooling-only JSON that records Forge input paths, raw
ability constructs, and their `NOT_IMPLEMENTED` semantic-lowering status.
Forge provenance identifies the input files; it is not Oracle or rules
authority. The definitions are `UnprofiledV1` and receive content-structure
validation only. This output does not admit or support any card.

Type-term classification uses the Comprehensive Rules snapshot pinned by the
accepted M4.1 CardDefinition Spec:
`wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`.
Subtype recognition is bounded to terms present in the selected scripts;
unknown characteristic terms fail with a card/face/line diagnostic.

The selected inputs, translated fields, observed abilities, and validation
results are summarized in [R1/W1 lowering report](R1_W1_LOWERING_REPORT.md).
