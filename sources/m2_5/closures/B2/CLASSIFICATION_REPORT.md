# M2.5.B2 Classification Report

Source package SHA-256: `99b33945a3e0c7b2982734e65f770715029ce6acd500104bde48e8466eed1a90`

## Closure counts

- REV3 historical families preserved: 216/216
- Shared OracleSemanticIdentity classifications: 402/402
- Deck-row projection: 441/441
- Reused OSIs: 23; additional row references: 39
- Total deck quantity: 600

## Lifecycle

- ACTIVE: 210
- ACTIVE_UNASSIGNED: 6
- SUPERSEDED: 0
- RETIRED: 0
- Terminal assignment edges: 1894
- Reviewed corrected authorities: 392; added edges: 1509; removed edges: 19

All 402 records were reviewed from the pinned card-side source. REV3 assignments are treated as historical proposals only; lexical scans are candidate generation and never terminal authority. Family-specific semantic predicates and reviewed correction decisions retain, add, or remove edges without changing historical family meanings. This is a Codex source-grounded review record, not a HUMAN_REVIEWED authorship claim.

## Semantic review accounting

- Lexical candidate edges inspected for empty-REV3 OSIs (non-authoritative): 1048
- Lexical candidates rejected by semantic review: 116
- Semantic edges admitted without a lexical candidate: 198

## Committed semantic boundaries

Every catalog family carries a family-specific B2_SEMANTIC_BOUNDARY_V1 definition with positive includes, relevant excludes, and the applicable §5.4 semantic dimensions. The catalog boundary is the terminal vocabulary authority; per-edge rationales explain how the pinned card-side evidence satisfies that boundary.

- Boundary-linked terminal assignment rationales: 1894/1894
- Boundary-linked delta rationales: 1913/1913

Lexical scans and family-specific predicates are disposable candidate/review tooling only. They do not authorize a terminal edge and are not a second rules engine or a replacement for the committed family boundary.

## Overlap and correction policy

Assignment policy: SEMANTIC_BOUNDARY_CLOSURE_V1 is additive. Every independently satisfied ACTIVE family boundary is assigned; a specialized family does not suppress a broader family, and a broader family does not suppress a specialized family. The target-destroy regression set requires cap.target, cap.destroy, and cap.target_destroy together when all three boundaries are satisfied.
- Reviewed targeted-destruction overlap cards: 17
- Additive saturation review scope: 210 ACTIVE family boundaries x 402 OSIs = 84,420 family/OSI pairs.
- Explicit source-grounded saturation additions: 538
- Confirmed semantic removals retained from the correction ledger: 8

## Regression anchors

- `cap.conditional_hexproof`: ACTIVE / REV3_LEGACY; historical members: 1; terminal usage: 1; card-side evidence is recorded for every assignment/change.
- `cap.life_drain`: ACTIVE / REV3_LEGACY; historical members: 1; terminal usage: 4; card-side evidence is recorded for every assignment/change.
- `cap.delayed_sacrifice`: ACTIVE_UNASSIGNED / REV3_LEGACY; historical members: 1; terminal usage: 0; card-side evidence is recorded for every assignment/change.
- `cap.countered_setup`: ACTIVE / REV3_LEGACY; historical members: 1; terminal usage: 4; card-side evidence is recorded for every assignment/change.
- `cap.copy_token_batch`: ACTIVE_UNASSIGNED / REV3_LEGACY; historical members: 1; terminal usage: 0; card-side evidence is recorded for every assignment/change.
- `cap.crew_alternative`: ACTIVE / REV3_LEGACY; historical members: 1; terminal usage: 3; card-side evidence is recorded for every assignment/change.
- `cap.mass_untap`: ACTIVE / REV3_LEGACY; historical members: 1; terminal usage: 5; card-side evidence is recorded for every assignment/change.
- `cap.tribal_permission`: ACTIVE / REV3_LEGACY; historical members: 1; terminal usage: 2; card-side evidence is recorded for every assignment/change.
- `cap.state_based_actions`: ACTIVE_UNASSIGNED / REV3_LEGACY; historical members: 1; terminal usage: 0; card-side evidence is recorded for every assignment/change.
- `cap.ordered_library_card`: ACTIVE / REV3_LEGACY; historical members: 5; terminal usage: 18; card-side evidence is recorded for every assignment/change.
- Current ACTIVE_UNASSIGNED families: cap.commander_zone_choice, cap.copy_token_batch, cap.delayed_sacrifice, cap.sacrifice_additional_cost, cap.state_based_actions, cap.token_replacement

## Evidence and downstream boundaries

Every assignment and retained change carries an OracleFieldLocatorV1 bound to the exact raw JSONL line and selected field. The classification identity is recomputed from the fixed ADR-0038 canonical-CBOR input.

OFFICIAL_RULE_CITATION_CLOSURE = BLOCKED (PENDING_B1_FINAL)
DECLARED_INTERACTION_MODEL_CLOSURE = BLOCKED
REV2_REUSE_RATIO_REPRODUCIBLE = BLOCKED
RANKING_UNCERTAINTY_PROPAGATION = BLOCKED
DECK_PAIR_LOCKED = NO
AUTHORITATIVE_RANKING_AVAILABLE = NO
M3_STARTED = NO

No B1 mapping, deck context, printing identity, format policy, or runtime semantics are used as shared classification authority.
