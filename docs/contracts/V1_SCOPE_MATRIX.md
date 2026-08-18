# V1 Scope Matrix

**Status:** blocked pending exact decks and snapshots
**Owner:** project owner + rules steward
**Last reviewed:** 2026-08-17

Replace every `TBD` before M4.

## Format identity

| Field | Locked value | Evidence |
|---|---|---|
| Product profile | official Commander, two-player environment | accepted scope |
| Comprehensive Rules | `TBD` | immutable ID/hash |
| Commander policy | `TBD` | immutable ID/hash |
| Banlist | `TBD` | immutable ID/hash |
| Oracle/card source | `TBD` | ID/hash and distribution policy |
| Mulligan | `TBD` | conformance case |
| Sideboards | none | accepted scope |
| Concession | `TBD` | environment contract |
| Loop/shortcut policy | `TBD` or explicit unsupported closure | ADR/cases |

## Deck manifests

| Seat | Deck ID | Commander ID | Exact manifest | Hash | Locked |
|---|---|---|---|---|---|
| Player 1 | `TBD-DECK-A` | `TBD` | `cards/manifests/TBD-DECK-A.toml` | `TBD` | no |
| Player 2 | `TBD-DECK-B` | `TBD` | `cards/manifests/TBD-DECK-B.toml` | `TBD` | no |

## Closure review

For both decks enumerate all faces, generated/referenced objects, mechanics,
layers, decisions/cardinalities, information transitions, native requirements,
and loops. Record implementation owner and evidence in a capability matrix.

Scope locks only when manifests are legal and immutable, closure is reviewed,
every required row has evidence, and no card relies on an unspecified excluded
behavior. Deck changes require a scope-impact report.
