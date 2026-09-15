# Manafold Research Inputs — 2026-09-15

**Status:** RESEARCH ONLY / NON-NORMATIVE / NOT SUPPORT AUTHORITY

**Repository baseline when collected:** `bd0b2461a74f9f4c35e5b52736c794a7d980959f`

This directory preserves the reviewed research inputs that informed the M3 entry tracker and adjacent M4/M5 planning. It is deliberately outside `docs/` so these research snapshots are not confused with normative/process documentation governed by the normative-document register.

The canonical live M3 scope tracker is GitHub Issue #178:

`https://github.com/chrismaghuhn/Manafold/issues/178`

Opening or storing these research records does **not** authorize M3 and does not create runtime, wire, Card IR, capability, benchmark, bundle, certification, or dataset support claims.

```text
RESEARCH != RUNTIME AUTHORITY
CENSUS != SUPPORT AUTHORITY
PROTOTYPE != ACCEPTED CONTRACT
WITNESS != SUPPORTED CARD
IMPLEMENTED != COVERED
COVERED != CERTIFIED
```

## Included reviewed snapshots

### `issue-129-conformance-interaction-research.md`

Reviewed M3 conformance / rule-composition research derived from Issue #129. Primary conclusion: M3.T0 should be a thin internal typed facade over the existing authoritative execution and proof machinery, not a second rules engine or public RulesCase protocol.

Role: **direct M3 Entry Decision input**.

### `standard-matchup-research.md`

Reviewed September 2026 Standard Mono-Red / Mono-White benchmark and capability census summary.

Key advisory counts for the leading asymmetric pair:

```text
pair_unique_cards = 26
direct_requirements = 93
candidate_capability_count = 92
potential_seams = 40
high_priority_interaction_candidates = 29
reviewed_obligations = 0
satisfied_interaction_evidence = 0
```

Role: **M3 dependency/interaction stress input and future M4 benchmark planning input**. It is not an implementation queue and does not freeze a deck.

### `card-ir-design-r2-research.md`

Reviewrevision R2 of the future Card IR design research, focused on continuous-effects evidence and controlled specialization of grouped operations.

Role: **future M4/content architecture research**. It is `DESIGN_ONLY`; it is not the currently accepted Card IR schema and creates no card support claim.

### `mnfl-v0.2-research.md`

Snapshot of the standalone MNFL 0.2 portable replay / presentation / player-trajectory prototype research.

Role: **future replay / ML interchange research**. It is explicitly not a Manafold integration, not a production wire/replay contract, and not a second rules engine.

### `SOURCE_PACKAGE_INVENTORY.txt`

Exact source-package filenames, byte sizes, SHA-256 identities, entry counts, and entry-path inventories for the three uploaded research packages.

## Source package identities

The exact uploaded source archives used for this snapshot were:

```text
manafold_standard_matchup_research(3).zip
size = 331604
sha256 = 04c8b52b65266e55b3d758b53211d6b75f0987aabb5d4afaa543e2b9498356d0
entries = 18

MNFL_Prototype_v0_2(2).zip
size = 594169
sha256 = 90baf52715a41b44ce905bd64e9e51ed25e4bfdb0c5bf34bb180fba23c59d59c
entries = 123

Manafold_Card_IR_Designentwurf_Reviewrevision_R2(2).zip
size = 124471
sha256 = a036e905032d436a930d9567334efebfec2536522d926413d39e3d018b401999
entries = 28

Issue-129 research Markdown source
size = 74630
sha256 = a33fffc2dcaf434a032eb86d9dde110a7c9cbca2139912486fad47dbdc70381b
```

The archive bytes themselves are not normative Manafold artifacts. This snapshot preserves their identities and reviewed conclusions; any later import of prototype implementation files must be a separate explicitly scoped change rather than silently entering the authoritative engine tree.

## Authority placement

### M3-relevant now

- capability taxonomy / semantic ownership research;
- Issue #129 conformance and interaction research;
- Standard requirement/capability census as a stress input;
- Issue #163 planning provenance;
- Issue #178 as the live candidate M3 entry/master tracker.

### Downstream research

- Card IR R2 belongs to M4/content design unless an M3 capability exposes a concrete missing general semantic primitive;
- MNFL v0.2 belongs to future replay/trajectory/interchange design and cannot override Manafold replay, wire, observation, information-state, or trajectory contracts.

## Preservation rule

Research artifacts may inform a reviewed decision. They do not become authoritative merely by being stored in this repository. Any recommendation promoted into an accepted contract must pass the normal ADR/contract/conformance process and be represented in the authoritative owning artifact.