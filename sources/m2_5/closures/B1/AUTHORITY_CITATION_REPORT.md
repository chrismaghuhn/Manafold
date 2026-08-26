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
| comprehensive_rules | OFFICIAL_RULES_TEXT | CITED | 66 exact rule citations (sections and subrules such as CR 702.143a resolved at their own `702.143a …` line), each identifier-derived and pinned inside the artifact effective 2026-08-07 |
| banned_restricted | OFFICIAL_LEGALITY_POLICY | CITED | page-lists root + Commander ban section; mechanically: 0 of 402 distinct resolved card names occur in that pinned section |
| commander_general | OFFICIAL_FORMAT_POLICY | CITED | format identity + Play Rules/Modifiers + Command Zone sections |
| commander_1v1 | OFFICIAL_FORMAT_POLICY | CITED | format identity + Play Rules/Modifiers sections |
| kaldheim_release_notes | OFFICIAL_UPDATE_NOTES | CITED | Release Information + "New Keyword: Foretell" (published 2021-01-22 per URL slug) |
| commander_legends_release_notes | OFFICIAL_UPDATE_NOTES | CITED | Release Information + "Returning Keyword: Partner" (published 2020-11-06 per URL slug) |
| magic_2013_release_notes | OFFICIAL_UPDATE_NOTES | NOT_REQUIRED_WITH_PROOF | resolution B via semantic dependency model; see §4 |

Every B1 record is cross-bound field-by-field to its REV3 register entry (URL,
artifact path, artifact digest, retrieval time, availability, HTTP/error status);
provenance swaps between real authorities are rejected by the verifier.

Locator scheme (mechanically re-executed against the pinned bytes):

```text
CR rules:      the locator is DERIVED from rule_identifier itself; section cites
               must resolve at '<num>.' headings, subrule cites (e.g. CR 702.143a)
               must resolve at their own '702.143a ' line; digest-checked.
HTML sections: unique raw byte fragment (offset, length, sha256, occurrence count == 1)
```

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
(no authoritative successor publication could be proven). Resolution B is established
by an **explicit authority→semantic dependency model** (primary evidence), with the
string scan demoted to a supplementary check:

1. **Semantic coverage (primary).** All 216 catalog requirement families are
   partitioned across the 66 CR citations plus the Commander-policy citations —
   every family carries exactly one covering citation edge into a CITED authority.
   The ten oracle-text mechanic concepts detected in the resolved rows (foretell,
   partner, crew, equip, living weapon, reconfigure, convoke, amass, populate,
   saga) each carry explicit edges. `magic_2013_dependency_edge_count = 0` is not
   asserted prose: the verifier recomputes the family partition from the pinned
   catalog, re-detects mechanics from the pinned deck-row surface, resolves every
   covering citation, and fails on any missing edge or any edge into
   magic_2013_release_notes. Semantic dependence on the Kaldheim notes, by contrast,
   is expressed exactly the same way (foretell edges), so the model cannot treat
   name-absence as independence for one authority and semantics for another.

2. **Supplementary string scan.** Re-executed over the checker-pinned canonical
   twelve-file surface catalog (`SURFACE_FILES` lives in the verifier, not in the
   proof record): manifest path-set must equal the canonical set, digests must
   match, and total hits must be zero. Omitting a manifest entry now fails via
   catalog deviation even when counters are adjusted consistently.

3. **Non-surface characterization.** The only `M13` bytes anywhere in the package
   are Scryfall printing candidates for basic lands and four cards inside
   `source/raw/default_cards_selected_REV3.jsonl`; printing-candidate bulk is not a
   dependency surface and consults no release-note authority.

Nothing was guessed, dropped for convenience, or rewritten historically: the REV3
record stays exactly as acquired; this closure is an additive record.

## 5. Identity separations (ADR 0041)

Source identity ≠ citation identity ≠ capability identity ≠ semantic ownership ≠
implementation ≠ certification. These are citations of official documents that
REV3 research relies on; they confer no implementation, support, or certification
status on any card, mechanic, or capability.

## 6. Verifier guarantees (executed)

Exact seven-authority universe (missing/duplicate/unknown rejected); terminality of
all seven; official role + Wizards-origin host enforcement; **field-exact cross-
binding of every record to its pinned REV3 register entry** (swapped provenance
between two real authorities fails); artifact digests resolved out of the verified
ZIP; full locator re-resolution with identifier-derived CR bindings (a valid
identifier over a foreign heading line fails); semantic dependency-model validation
(complete family partition, detected-mechanic edges, zero magic_2013 edges);
supplementary scan re-execution against the checker-pinned surface catalog
(omission fails even with self-consistent counters); closure agreement (counts,
gate statuses, remaining gates BLOCKED, deck/ranking/M3 flags false); digest binding
of citations file and this report. Negative self-test: positive control PASS plus 14
adversarial fixtures, all rejected by real checker logic.

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
