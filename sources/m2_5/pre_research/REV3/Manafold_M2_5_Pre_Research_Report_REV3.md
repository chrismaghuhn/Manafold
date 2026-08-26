# Manafold M2.5 Pre-Research REV3

## Execution status

`PRE_RESEARCH_COMPLETE = NO`  
`M2.5 STARTED = NO`  
`DECK_PAIR_LOCKED = NO`  
`M3 STARTED = NO`

This package is a completed acquisition/generation attempt with fail-closed
research gates. It is not a deck-selection authority and it does not modify
the Manafold repository.

## Reproducible input snapshot

- Six locked source snapshots; 441 classification rows; total quantity 600.
- Each deck contributes quantity 100.
- Current `master` was resolved once as `05bd341635ba2150d721659b15b62c5ad311637f`.
- Final drift check: `BLOCKED`.
- Scryfall compressed and uncompressed bulk hashes, exact selected JSONL raw
  record hashes, and normalized REV3 hashes are recorded in the acquisition
  manifest and source record index.
- Official authority artifacts: six hashed downloads; the Magic 2013 release
  note URL returned HTTP 404 and is recorded as unavailable, not guessed.

## Identity and source evidence

- `DeckRowIdentity` is unique by deck plus source row.
- `OracleSemanticIdentity` is source-provided card-level `oracle_id` only.
- Face-level IDs are preserved separately; no composite face identity is
  created.
- Concrete `PrintingIdentity` is populated only for an explicit set plus
  collector-number source row. Name-only and set-only rows do not receive a
  selected printing ID.
- Result: `441/441` rows resolved to a
  unique supported card-level OracleSemanticIdentity; `0` remain unresolved.

Unresolved rows:



Repeated OracleSemanticIdentity values across different deck rows are allowed;
the validator observed 23 reused identities and no duplicate DeckRowIdentity.

## Classification and interaction closure

- Shared authorities: `402`
  per OracleSemanticIdentity.
- Terminal ranking authorities: `8`;
  inherited estimates remain non-authoritative.
- `interaction-model.v1` is explicitly scoped to unary/card-specific,
  binary, directional, and explicitly reviewed higher-order cases. It does not
  claim arbitrary future N-way Magic completeness.
- Interaction candidate universe: `15679`;
  current disposition closure is BLOCKED because candidates remain
  `AMBIGUOUS_REQUIRES_REVIEW`.

## Blocking gates

- `OFFICIAL_RULE_CITATION_CLOSURE`
- `CLASSIFICATION_REFERENCE_CLOSURE`
- `DECLARED_INTERACTION_MODEL_CLOSURE`
- `REV2_REUSE_RATIO_REPRODUCIBLE`
- `RANKING_UNCERTAINTY_PROPAGATION`
- `MASTER_DRIFT`

## Ranking status

The REV2 reuse-ratio meaning was not reconstructable from the packaged
evidence. REV3 therefore reports
`REV2_REUSE_RATIO_REPRODUCIBLE = NO` and
`REV2_RANKING_FORMULA_FULLY_REPRODUCIBLE = NO`, introduces no replacement
formula, persists no authoritative scores, and keeps exact `Fraction`
arithmetic as the declared representation for a future closed run.

Lower-bound, upper-bound, and plausible-alternative scenario records are
present. They contain no fabricated scores or rank intervals while upstream
closures remain blocked.

## Blocking reasons

- REV2 inherited classifications are not terminal REV3 review authorities for all OracleSemanticIdentity values.
- interaction-model.v1 has unresolved candidate dispositions.
- REV2 reuse ratio and full ranking arithmetic are not yet reproducible from canonical inputs.

## Verification evidence

- Structural validator: `PASS` with no
  structural failures; research status `BLOCKED`.
- Independent arithmetic validator: `PASS`.
- Negative-test runner execution: `PASS`.
- Required adversarial negative fixtures: `PASS` across `12` real mutation cases.
- Two clean offline generator runs: `PASS` with
  `54` byte-identical files.

See `completion_status_REV3.json` for the machine-readable milestone state.
