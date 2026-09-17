# Standard Mono-Red / Mono-White Research Snapshot — 2026-09-15

**Status:** RESEARCH ARTIFACT / NON-NORMATIVE / NOT A CAPABILITY REGISTRY / NOT A CERTIFIED BUNDLE

**Source package identity:**

```text
filename = manafold_standard_matchup_research(3).zip
size = 331604
sha256 = 04c8b52b65266e55b3d758b53211d6b75f0987aabb5d4afaa543e2b9498356d0
entries = 18
```

The package contains the full Markdown/HTML report, advisory census JSON, all candidate deck transcriptions, taxonomy research input, validation report, and offline builder. This repository snapshot records the reviewed conclusions and source identity; it does not promote the advisory census into Manafold support authority.

## Primary recommendation

Keep the researched Mono-Red × Mono-White pair as a serious candidate for a later first **asymmetric gameplay benchmark**, but do not use it as the smallest first throughput/reference workload.

Provisionally prefer an **unchanged Mono-Red mirror** for the earlier full-game reference/throughput role, subject to its own interaction review and later normal certification requirements.

This is a structural scope recommendation, not a performance claim.

## Current Standard framing

The research checked current September 2026 Standard policy rather than assuming an older rotation schedule. It treats Mono-Red and Mono-White as recognizable, externally sourced benchmark decks rather than claiming this pairing is the September Tier-1 metagame matchup.

The leading exact pair is:

```text
R1 = Endo Takatomo Mono-Red burn/Ojer aggro
     event date 2026-09-12

W1 = Sawada Atsushi Mono-White Auras
     event date 2026-09-05
```

Both are preserved as exact real-world list candidates. Research derivatives that remove semantic complexity are explicitly labeled as unselected derivatives and are not silently substituted for the source lists.

## Leading-pair census

```text
red_main_deck_cards = 60
white_main_deck_cards = 60

red_unique_cards = 12
white_unique_cards = 14
pair_unique_cards = 26

red_unique_nonlands = 10
white_unique_nonlands = 12

reachable_direct_requirement_assertions = 93
direct_candidate_capability_roots_pair_union = 85
red_transitive_candidate_capability_closure = 69
white_transitive_candidate_capability_closure = 77
pair_candidate_capability_closure = 92

shared_candidate_capabilities = 54
red_only_candidate_capabilities = 15
white_only_candidate_capabilities = 23

explicit_dependency_edges = 203
screened_potential_seams = 40
pair_applicable_research_seams = 38
supplementary_generalized_boundary_seams = 2
high_priority_interaction_candidates = 29

reviewed_interaction_obligations = 0
satisfied_interaction_evidence = 0
```

These counts describe the research partition. They are not intrinsic feature counts, coverage percentages, registry entries, lifecycle states, or certification evidence.

## Why the asymmetric pair is already semantically substantial

The white side adds several difficult systems together despite having only 14 unique cards. Research examples include:

- Aura attachment and attachment legality;
- continuous modification and characteristic dependencies;
- Role creation and Role uniqueness;
- triggered/reflexive/delayed trigger timing;
- Saga sequencing;
- negative counters;
- graveyard recovery;
- exile duration linked to another object;
- ward/lifelink assigned to an enchanted host;
- SBA chains involving creature death and Aura loss;
- hidden scry information;
- source-scoped and turn-scoped history.

The red side also contains nontrivial semantics. In particular, the Ojer package is not merely a damage multiplier: source color/controller, recipient, damage category, power, object lifecycle, transform/return behavior, and qualifying noncombat damage history create reusable semantic requirements.

## Benchmark-role split

Research comparison:

```text
R1 × R1
  unique card names = 12
  candidate capability closure = 69

R1 × W1
  unique card names = 26
  candidate capability closure = 92
```

Advisory role split:

```text
earlier full-game reference / throughput candidate
= R1 × R1 mirror

later first asymmetric gameplay benchmark candidate
= R1 × W1
```

Neither workload is frozen by this research.

## M3 implication

Do **not** turn the 92-node advisory closure into an M3 implementation queue.

Use the deck/capability census to expose the dependency frontier underneath the content. The useful early semantic foundations indicated by the workload include areas such as:

```text
state / identity / zone incarnation / LKI
turn / phase / priority / forced progress
Decision composition and soundness/completeness
casting / costs / payment
target legality
semantic event pipeline
replacement / prevention
triggers
SBA fixed-point behavior
damage / life / source-scoped history
continuous characteristics / copy foundations
combat declarations / legality / damage
projection / knowledge / noninterference
checkpoint / fork / replay
```

Only the exact reviewed subset selected by the M3 Entry Decision belongs to the Initial Semantic Foundation.

## Interaction implication

The research produced 40 `POTENTIAL_SEAM` records and deliberately promoted none to authoritative obligations.

Use the accepted lifecycle:

```text
POTENTIAL_SEAM
→ REVIEWED_OBLIGATION
→ SATISFIED_EVIDENCE
```

High-value seam families observed by the benchmark include:

- combat × keywords / characteristic changes;
- damage × source attribution / history;
- removal × Aura/exile duration;
- trigger/reflexive trigger sequencing;
- replacement / prevention;
- SBA chains;
- tokens / Roles / counters;
- continuous effects;
- costs / restricted mana;
- graveyard/exile return;
- hidden-information operations;
- cleanup / duration semantics.

The seam set is a risk map, not a Cartesian test requirement.

## M4 and benchmark implications

Preserve exact real-world list identity rather than silently simplifying lists for implementation convenience.

A future frozen benchmark must bind at least the appropriate exact identities for:

```text
rules snapshot
Oracle/card-source snapshot
format policy snapshot
card/generated-object bundle
deck manifests
RNG contract / seed set
engine build
protocol identities
benchmark workload identity
agent/seat binding policy
```

Rotation or later legality changes should not silently rewrite an already frozen historical benchmark identity.

## Evidence limits

This research did **not** perform:

```text
engine conformance
legal full-game simulation
replay execution
bundle certification
performance benchmark
immutable complete primary Oracle capture
```

Deck arithmetic/transcription, graph references/cycles, report structure, and research artifact checks are not Magic correctness evidence.

Sideboards and alternate-list legality have narrower evidence boundaries than the leading main-deck review.

Therefore:

```text
FREEZE_THIS_PAIR_NOW = NO
CAPABILITIES_REGISTERED = NO
REGISTRY_MODIFIED = NO
CARDS_IMPLEMENTED = NO
BUNDLE_CERTIFIED = NO
M3_STARTED = NO
M3_AUTHORIZED = NO
```

## Reviewed conclusion

```text
STANDARD_MATCHUP_RESEARCH_REVIEW = APPROVE
EARLY_REFERENCE_CANDIDATE = PROVISIONAL_R1_MIRROR
ASYMMETRIC_BENCHMARK_CANDIDATE = CONDITIONAL_R1_X_W1
USE_AS_M3_STRESS_INPUT = YES
USE_AS_M3_SCOPE_COUNT = NO
USE_AS_SUPPORT_CLAIM = NO
```

The source package also carries the capability-taxonomy research used by the report. Its example capability identifiers remain research candidates until separately reviewed and admitted by Manafold.