# V1 Scope Matrix

**Status:** exact two-deck selection locked; capability closure pending
**Owner:** project owner + rules steward
**Last reviewed:** 2026-09-12

The authoritative pre-M4 deck lock is
[`sources/m2_5/scope/exact_two_deck_scope_lock.v1.json`](../../sources/m2_5/scope/exact_two_deck_scope_lock.v1.json).
It is a source-bound M2.5 scope selection artifact, not a Card Definition or
certified Bundle Manifest. Its exact source package is the private REV3 archive
with SHA-256
`99b33945a3e0c7b2982734e65f770715029ce6acd500104bde48e8466eed1a90`.

Replace remaining unrelated `TBD` values only when their own evidence closes.

## Format identity

| Field | Locked value | Evidence |
|---|---|---|
| Product profile | official Commander, two-player environment | accepted scope |
| Comprehensive Rules | `baseline/REV2/Authority_Snapshots/Comprehensive_Rules_manifest_2026-08-19.json` (`member sha256=51245278b16ae4c84da32e78a8399ec82d26a7af0177e732e44425233b495254`) | raw rules SHA-256 `047f8944fc1c1d18d9c2a3daa28bac99531e95b9133c066a6cfb05718775b85a` |
| Commander policy | `baseline/REV2/Authority_Snapshots/Commander_policy_and_banlist_2026-08-23.json` (`member sha256=430a87db17fe2d8e8737da44befe9ca7d557b806826e126e712ce856f3cdfec9`) | official Commander 1v1; deck size 100; two players |
| Banlist | `baseline/REV2/Authority_Snapshots/Commander_policy_and_banlist_2026-08-23.json` (`member sha256=430a87db17fe2d8e8737da44befe9ca7d557b806826e126e712ce856f3cdfec9`) | exact-name check PASS; no selected card matches |
| Oracle/card source | `baseline/REV2/Authority_Snapshots/Scryfall_bulk_manifest_2026-08-21.json` (`member sha256=7b0ccc8491992085b90c1789329b945c02299057e93491e94f962078448d9798`) | bulk snapshot SHA-256 `a33d1b0a56d34b28832fd00bff83258c1013fef13588b3c59dcfd3ef79f16c2e`; selected Oracle/source records are bound in the lock artifact |
| Mulligan | `TBD` | conformance case |
| Sideboards | none | accepted scope |
| Concession | `TBD` | environment contract |
| Loop/shortcut policy | `TBD` or explicit unsupported closure | ADR/cases |

## Deck manifests

| Seat | Deck ID | Commander ID | Exact manifest | Hash | Locked |
|---|---|---|---|---|---|
| Player 1 | `m2-5/token-triumph` | Token Triumph — Emmara, Soul of the Accord (`oracle_semantic_identity=c65ba242-3369-48a9-864f-1b1f85238f67`) | `sources/m2_5/scope/exact_two_deck_scope_lock.v1.json#/decks/0` | `a61e6cd78c441346e8c52eff8af08cff53698e2af32797db07bedbadcabb1c93` | yes |
| Player 2 | `m2-5/grave-danger` | Grave Danger — Gisa and Geralf (`oracle_semantic_identity=20e94d0f-6887-45f8-a137-877011d786c6`) | `sources/m2_5/scope/exact_two_deck_scope_lock.v1.json#/decks/1` | `93d59175ba0d9df6767d6f861a4aebfbaa65895f744b52c2a2f202950806ba45` | yes |

## Selection status

```text
EXACT_TWO_DECK_SELECTION       = PASS
DECK_PAIR_LOCKED               = YES
PLAYER_1                      = Token Triumph
PLAYER_2                      = Grave Danger
AUTHORITATIVE_RANKING_AVAILABLE = NO
C_PASS                         = BLOCKED
M2_5_FINAL                     = NOT_YET
M3                            = NOT_AUTHORIZED
```

This is an explicit maintainer scope-selection decision based on the existing
source-grounded research envelope. It is not an authoritative ranking result.
The selected pair is locked for the next finite M2.5 capability/census block;
the M2.5 final closure requirements below remain open.

## Closure review

For both decks enumerate all faces, generated/referenced objects, mechanics,
layers, decisions/cardinalities, information transitions, native requirements,
and loops. Record implementation owner and evidence in a capability matrix.

The pair-selection lock above is not the M2.5 final milestone claim. M2.5 final
still requires manifests to be legal and immutable, closure to be reviewed,
every required row has evidence, and no card relies on an unspecified excluded
behavior. Deck changes require a scope-impact report.
