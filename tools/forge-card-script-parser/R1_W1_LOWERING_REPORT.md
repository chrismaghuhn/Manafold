# R1/W1 Forge characteristic lowering report

The lowering run used only the 26 source paths in
[R1/W1 parser validation](R1_W1_REPORT.md), read from Forge commit
`17c1ba92149b84127749bf84c231ed75107df1a2`. The command checks the checkout
revision and verifies the selected Forge files have no local edits. The type
term categories use the accepted M4.1 Spec's pinned Comprehensive Rules
snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`.

## Outcome

```text
R1_CANDIDATES = 12
W1_CANDIDATES = 14

CHARACTERISTICS_LOWERED = 26
CHARACTERISTICS_BLOCKED = 0

MULTIFACE_DEFINITIONS = 1 (Ojer Axonil, Deepest Might; 2 ordered faces)
UNLOWERED_ABILITY_CONSTRUCTS = 96

CARD_IR_VALIDATION = PASS (canonical manifest, content identity, matching
                          Forge source provenance, validation-only preflight)
DETERMINISTIC_OUTPUT = PASS (two pinned-source runs produced byte-identical
                             manifest, provenance, and metadata files)
```

The generated manifest has content identity
`f60ab9a105fdcaff495af0c52b2158ed075519bf1da46937893f013eeef27126`. Ojer's
second face, Temple of Power, remains face key 1 of that one definition. It is
not another deck candidate. `ManaCost:no cost` maps to absent printed mana cost
(`None`); hybrid `WU WU` and `GW` remain ordered, single hybrid symbols.
Candidate-local definition IDs 1–26 follow the tool manifest order (R1, then
W1).

All definitions bind to `UnprofiledV1`, have no ability identities, references,
or capability requirements, and passed existing structural content validation
with `ValidationOnly` authorization. This run adds no Basic Land admission and
does not claim any card support or executable semantics.

## Unlowered source constructs

All source entries are retained in tooling metadata with their raw values and
source face. Ability structure, when parsed, is descriptive only; every entry
has `semantic_lowering: NOT_IMPLEMENTED`.

```text
A: 12
T: 14
R: 3
S: 8
K: 21
SVar: 38
TOTAL: 96
```

The two issue aliases remain attached to the normalized candidates:
`A Most Helpful Weaver` → `Origin of Spider-Man` and `Wonderweave Aerialist` →
`Skyward Spider`. Forge commit/path/byte digest provenance is kept separate
from the pinned Comprehensive Rules type vocabulary and makes no Oracle
authority claim.

## Candidate artifacts

- `candidates/r1w1/content-contract.v1.cbor` — canonical existing typed Card IR manifest.
- `candidates/r1w1/provenance.v1.cbor` — existing provenance catalog format, identifying the exact Forge input records.
- `candidates/r1w1/lowering-metadata.v1.json` — deck order, aliases, face summaries, source identities, and all unlowered constructs.

The artifacts contain no copied Forge scripts or Oracle text. The metadata
also preserves other non-characteristic Forge fields without interpreting
them.
