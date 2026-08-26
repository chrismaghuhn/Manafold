# M2.5.B1 — Official Authority Citation Closure Report

- **Slice:** B1 (owns exactly `OFFICIAL_RULE_CITATION_CLOSURE`)
- **Input universe:** the seven REV3 authority records, treated as closed
- **Machine-readable evidence:** [`official_authority_citations.v1.json`](official_authority_citations.v1.json), [`official_authority_citation_closure.v1.json`](official_authority_citation_closure.v1.json)
- **Executable verifier:** `scripts/check_m2_5_b1_authority_citations.py` (+ `--negative-self-test`)

## 1. Preconditions executed

```text
python scripts/check_m2_5_master_drift.py            -> MASTER_DRIFT_CLOSURE_CHECK = PASS
python scripts/check_m2_5_master_drift.py --verify-archive
                                                     -> ARCHIVE_PREFLIGHT = PASS
```

All REV3 payload used by this slice was consumed from the SHA-256-verified private
archive (`99b33945…1a90`), not from an ad-hoc extraction.

## 2. Terminal outcomes — 7 / 7

| Authority | Role | Status | Anchor summary |
|---|---|---|---|
| comprehensive_rules | OFFICIAL_RULES_TEXT | CITED | 23 exact `CR xxx[.x[a-z]]` rule citations, each resolved at a pinned heading line inside the artifact effective 2026-08-07 |
| banned_restricted | OFFICIAL_LEGALITY_POLICY | CITED | page-lists root + Commander ban section; mechanically: 0 of 402 distinct resolved card names occur in that pinned section |
| commander_general | OFFICIAL_FORMAT_POLICY | CITED | format identity + Play Rules/Modifiers + Command Zone sections |
| commander_1v1 | OFFICIAL_FORMAT_POLICY | CITED | format identity + Play Rules/Modifiers sections |
| kaldheim_release_notes | OFFICIAL_UPDATE_NOTES | CITED | Release Information + "New Keyword: Foretell" (published 2021-01-22 per URL slug) |
| commander_legends_release_notes | OFFICIAL_UPDATE_NOTES | CITED | Release Information + "Returning Keyword: Partner" (published 2020-11-06 per URL slug) |
| magic_2013_release_notes | OFFICIAL_UPDATE_NOTES | NOT_REQUIRED_WITH_PROOF | resolution B; see §4 |

Locator scheme (both mechanically re-executed by the verifier against the pinned bytes):

```text
CR rules:      line_number_1based + expected_heading_prefix + sha256(stripped heading line)
HTML sections: unique raw byte fragment (offset, length, sha256, occurrence count == 1)
```

A bare heading name is never sufficient: every HTML citation pins exact archive bytes.

## 3. Why these dependencies (derived, not asserted)

Deck rows resolve to exactly three products — KHC (150 rows), NEC (147), SCD (144) —
with six commanders (Kotori; Lathril; Gisa and Geralf; Ranar; Emmara; Chishiro).
Requirement-family usage and oracle-text mechanics drive each authority's
`dependent_research_requirements` / family links, e.g.:

- `cap.foretell` (10 map rows) + `cap.private_face_down_exile` (9) plus 33 foretell
  oracle-text occurrences → Kaldheim notes + CR 702.143;
- vehicles/crew cluster (`cap.vehicle` 8, `cap.crew` 2, …) → CR 301 / CR 702.122;
- equipment clusters → CR 702.6 / 702.92 / 702.151;
- partner (4 oracle-text occurrences) → CL notes + CR 702.124;
- token/counters/layers/SBA families → CR 111 / 122 / 613 / 704.

Per the slice contract this records dependence only; it does **not** advance
classification review (`CLASSIFICATION_REFERENCE_CLOSURE` remains BLOCKED).

## 4. Magic 2013 404 — resolution B (NOT_REQUIRED_WITH_PROOF)

The acquisition-time HTTP 404 is handled explicitly. Resolution A was not available
(no authoritative successor publication could be proven); resolution B was established
mechanically over the complete pinned payload (12 surface files, digest-manifested):

```text
patterns (case-insensitive): magic_2013, magic-2013, "magic 2013", update-bulletin
surfaces: deck rows (441) · OracleSemanticIdentity evidence · classification
authorities (402) · requirement families (216 ids / 470 map rows) · interaction
candidates (15,679) · ranking inputs
total relevant dependency hits: 0
```

The only `M13` byte hits anywhere in the package are Scryfall printing candidates for
basic lands and four cards inside `source/raw/default_cards_selected_REV3.jsonl`;
printing-candidate bulk is not one of the six dependency categories and consults no
release-note authority. This characterization is recorded in the citations JSON.

Nothing was guessed, dropped for convenience, or rewritten historically: the REV3
record stays exactly as acquired; this closure is an additive record.

## 5. Identity separations (ADR 0041)

Source identity ≠ citation identity ≠ capability identity ≠ semantic ownership ≠
implementation ≠ certification. These are citations of official documents that
REV3 research relies on; they confer no implementation, support, or certification
status on any card, mechanic, or capability.

## 6. Verifier guarantees (executed)

Universe completeness/duplication/unknown-authority rejection; terminality of all
seven; official role + Wizards-origin host enforcement; artifact digests resolved out
of the verified ZIP; every locator re-resolved inside pinned bytes; zero-dependency
re-scan reproduction with digest-matched manifest; closure agreement (counts, gate
statuses, remaining gates BLOCKED, deck/ranking/M3 flags false); digest binding of
citations file and this report. Negative self-test: positive control PASS plus 12
adversarial fixtures, all rejected (see `verification/b1_checker_record.json`).

## 7. State after B1

```text
M2.5.A                              = COMPLETE
MASTER_DRIFT                        = PASS

OFFICIAL_RULE_CITATION_CLOSURE      = PASS

CLASSIFICATION_REFERENCE_CLOSURE    = BLOCKED
DECLARED_INTERACTION_MODEL_CLOSURE  = BLOCKED
REV2_REUSE_RATIO_REPRODUCIBLE       = BLOCKED
RANKING_UNCERTAINTY_PROPAGATION     = BLOCKED

DECK_PAIR_LOCKED                    = NO
AUTHORITATIVE_RANKING_AVAILABLE     = NO
M3_STARTED                          = NO

M2.5                                = ACTIVE   (project milestone state)
```

Historical REV3 `M2_5_STARTED = NO` remains a point-in-time pre-research fact and is
not rewritten.
