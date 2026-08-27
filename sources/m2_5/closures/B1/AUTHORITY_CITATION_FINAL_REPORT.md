# M2.5.B1.Final — Terminal Official Rule Citation Closure Report

This additive B1.Final report preserves historical B1 V2/V1 meaning and binds the terminal semantic scope to the immutable B2 snapshot.

## Semantic closure

- Exact B2 terminal families reviewed: **210 ACTIVE / 210 required**.
- Review basis: every committed B2 boundary field (`includes`, `excludes`, objects, action/event, timing, visibility, eligibility/duration, choices, ownership/control, numeric/counter effects, identity effects, and rule dependency).
- Lexical markers and scans are candidate/audit diagnostics only and are not authority proof.
- Families changed relative to inherited B1 V2 mapping: **138**.
- Required family-level citation edges after review: **380**.
- Multi-citation families: **125**.
- New/re-required citation edges relative to V2: **188**.
- Removed/replaced inherited V2 edges: **18**.

## Fixed semantic regression anchors

- `cap.alternate_cast_zone` → CR-601-casting-spells
- `cap.artifact_animation` → CR-301-artifacts, CR-613-continuous-effects
- `cap.attack_mana` → CR-508-declare-attackers, CR-605-mana-abilities
- `cap.continuous_ability` → CR-604-static-abilities, CR-611-continuous-effects, CR-613-continuous-effects
- `cap.copy` → CR-111-tokens, CR-707-copying-objects
- `cap.flash` → CR-702-8-flash
- `cap.flashback` → CR-404-graveyard, CR-601-casting-spells, CR-702-34-flashback
- `cap.improvise` → CR-702-126-improvise
- `cap.loyalty` → CR-122-counters, CR-306-planeswalkers, CR-602-activated-abilities, CR-606-loyalty-abilities
- `cap.loyalty_activation_rules` → CR-117-timing-priority, CR-306-planeswalkers, CR-602-activated-abilities, CR-606-3-loyalty-activation
- `cap.mass_untap` → CR-701-26-tap-untap
- `cap.modified_predicate` → CR-700-9-modified
- `cap.token_or_counters` → CR-111-tokens, CR-122-counters, CR-700-2-modes
- `cap.tribal_permission` → CR-205-type-line, CR-601-casting-spells

## Citation changes

New official CR citations relative to the previous V3 authority register: **CR-105-colors, CR-500-general, CR-606-loyalty-abilities, CR-606-3-loyalty-activation, CR-611-continuous-effects, CR-700-9-modified, CR-702-8-flash, CR-702-34-flashback, CR-702-126-improvise, CR-707-copying-objects**.
Replaced/removed inherited citation IDs: CR-115-targets, CR-117-timing-priority, CR-301-artifacts, CR-404-graveyard, CR-406-exile, CR-604-static-abilities, CR-700-general, CR-701-3-attach, CR-702-143a-foretell, CR-706-copiable-values.
The inherited `CR-706-copiable-values` record was removed from V3 because its locator identified `706. Rolling a Die`; copy dependencies now bind `CR 707. Copying Objects`.

## Scope and gates

B2 remains an immutable input: 216 families, 402 classifications, 1883 terminal assignments, 441 projections, 210 ACTIVE, and six ACTIVE_UNASSIGNED. The B2 historical citation handoff remains `BLOCKED / PENDING_B1_FINAL`.

The B1.Final terminal transition is `CLASSIFICATION_REFERENCE_CLOSURE = PASS` and `OFFICIAL_RULE_CITATION_CLOSURE = PASS`. Interaction modeling, REV2 reuse reproducibility, ranking uncertainty propagation, deck-pair locking, authoritative ranking, and M3 remain blocked/not started.

The machine-readable full review is embedded in `semantic_dependency_review` in `official_authority_citations.v3.json`; the executable verifier rejects omitted families, uncovered domains, inherited unrelated edges, altered B2 boundaries, and anchor regressions.
