# Pre-M3 Remediation Batch A: State Closure and Canonicalization

**Status:** proposed for Batch-A review

## Goal

Close the confirmed authoritative state-closure defects FND-001, FND-003,
FND-004, and FND-005, and resolve the disposition of FND-002 and FND-006
without changing M3 semantics, public wire bytes, schemas, RNG behavior, or
digest domains.

## Authority and disposition hypothesis

The current repository is authoritative. `docs/03_NORMATIVE_HIERARCHY.md` is
absent; the active hierarchy is `docs/NORMATIVE_HIERARCHY.md`.

- FND-001 uses the existing V3 Commander layout and its explicit ascending,
  duplicate-free membership contract. The validator will reject a designation
  vector that is not strictly ascending. Digest conversion remains unchanged.
- FND-002 is `BLOCKED_CONTRACT_AMBIGUITY` unless the current normative
  information contract proves a unique cross-field chronology. Existing
  validation of per-field sequence bounds and ordered history is not evidence
  for an additional acquisition/current/history/invalidation ordering rule.
- FND-003 is confirmed. Every `ZoneLocation.player` in active current facts,
  active history, retired last-known facts, and retired history must belong to
  the authoritative player set.
- FND-004 is confirmed. A pending authoritative `SelectPlayer` candidate must
  name a declared player; visible/trusted equality does not establish state
  closure.
- FND-005 is confirmed in one direction. Every retired knowledge record must
  have the matching retired opaque identity marker. A retired opaque marker may
  exist without retained knowledge because the lifecycle contract permits
  identity retirement when no knowledge record exists.
- FND-006 is split. Empty ordered-zone keys and their declared-player
  references are a confirmed state-closure defect. The relationship between
  `ZonePosition` payloads and vector indices is recorded separately as
  contract ambiguity unless the current authority supplies an exact mapping;
  no positional rule will be invented.

## Architecture

`mtgml-state::validate_engine_state` remains the sole cross-component
validation owner. The state validation coordinator will pass the authoritative
player set to shared location-reference checks, and the existing decision,
knowledge, format, and zone validation segments will report their existing
closed violation families. No normalization occurs after invalid state has
entered the authoritative model. `EngineState::canonical_digest_bytes` and
`EngineState::digest` continue to reject invalid state through the validator,
while preserving the existing V3 input layout and digest domain.

The implementation will add focused state-level regressions first. Each
confirmed defect will demonstrate that the baseline accepts the malformed
state, then the smallest validator change will make the same test pass. Tests
will also preserve the explicitly allowed retired-identity-without-knowledge
case and valid ordered/Commander representations.

## Verification

The RED/GREEN loop covers the owning `mtgml-state` tests. Affected package
tests will then run for decision, observation, rules, environment, and
conformance only where dependency impact requires them. The final verification
includes Rust formatting, workspace check, workspace clippy, workspace tests,
and the repository's applicable integration gates. Results remain separately
classified as `PASS`, `FAIL`, `NOT_RUN`, or `BLOCKED`; M3 remains unauthorized.

## Scope exclusions

This design does not add real Magic rules, cards, decks, Card IR behavior, M3
work, Decision protocol redesign, replay redesign, schema evolution, digest
domain changes, RNG changes, modularization, or unrelated audit fixes.
