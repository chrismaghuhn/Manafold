# Issue #129 Research Snapshot — M3 Conformance, Interaction Coverage, and Rule Composition

**Status:** REVIEWED RESEARCH INPUT / NON-NORMATIVE

**Research date:** 2026-09-15

**Research baseline:** `bd0b2461a74f9f4c35e5b52736c794a7d980959f`

**Review outcome:** `APPROVE`, with `0 BLOCKER / 0 MAJOR / 0 MINOR / 0 NIT`.

The original full research source used for this snapshot had SHA-256:

`a33fffc2dcaf434a032eb86d9dde110a7c9cbca2139912486fad47dbdc70381b`

No new repository tests were executed by the research itself. Statements about existing foundation proof surfaces are based on source inspection plus already-committed exact-head foundation evidence.

## Executive conclusion

The smallest safe M3 conformance architecture is:

```text
reviewed case expectations
        ↓
complete validated scenario
        ↓
existing trusted controller / player endpoint path
        ↓
authoritative Rust RulesKernel
        ↓
actual transition + perspective products
        ↓
thin mtgml-conformance facade
        ↓
independent exact assertions
```

M3.T0 should therefore be a **thin internal typed Rust facade** over proof/execution primitives Manafold already has. It must not become:

- a second rules engine;
- a generic rules DSL;
- a public `RulesCaseV1` promise;
- a replacement replay/noninterference/legal-space framework;
- a place where targets, costs, payments, triggers, replacement effects, SBAs, layers, copy semantics, or combat legality are calculated independently.

The key M3 gap is **composition and ergonomics**, not missing low-level proof machinery.

## Current reusable foundation identified by the research

The current repository already contains reusable infrastructure for:

```text
exact transition comparison
complete rejection fingerprints / nonmutation
legal-space soundness and completeness
paired-state information noninterference
checkpoint parity
fork parity
replay reprojection/parity
bounded legal-space exploration
first-divergence diagnostics
trusted reproducible failure packets
targeted information-leak mutants
```

General M3 Magic interaction evidence remains `NOT_APPLICABLE_YET` until real capabilities exist.

## Original #129 findings — updated disposition

```text
F01 = PARTIALLY_RESOLVED
      remaining work becomes M3.T0

F02 = STILL_VALID_REFRAMED
      Requirement → Capability → seam → obligation → evidence

F03 = STILL_VALID
      first meaningful Rust semantic smoke belongs WITH S1

F04 = RESOLVED / SUPERSEDED
      governance/maintainer status concern, not T0 work

F05 = STILL_VALID
      new event families need repeated/mixed/pipeline evidence

F06 = PARTIALLY_RESOLVED
      no property framework in T0; introduce only for concrete value

F07 = STILL_VALID
      deterministic semantic campaigns belong later in M3

F08 = PARTIALLY_RESOLVED
      reuse targeted validity-gated mutant philosophy

F09 = PARTIALLY_RESOLVED
      avoid premature impact-based CI selection machinery

F10-F14 = SAFE_TO_DEFER / STILL_VALID
```

## M3.T0 minimum shape

T0 should support one obvious case-authoring path capable of expressing:

```text
complete validated setup
multi-step scenario
exact current authoritative Decision
explicit submitted response
accepted/rejected outcome
ordered authoritative events
complete StateDelta
independently authored complete resulting EngineState where appropriate
state digest
next Decision
EpisodeStatus
per-player PlayerStep/projection
complete rejection fingerprint
checkpoint/restore parity
fork parity
replay parity
deterministic first-divergence diagnostics
trusted failure-packet integration where practical
```

T0 is repository-internal and experimental.

```text
M3.T0 = proof / conformance infrastructure
M3.T0 != first Magic capability
```

A crucial review rule is:

```text
HARNESS SELF-TEST
may construct expected values from actual output when testing the comparator

MAGIC CONFORMANCE CASE
must not derive semantic expected values from production execution
```

## Scenario construction policy

Keep these concepts distinct:

```text
structurally valid
case-valid
historically reachable
```

Default strategy:

```text
structural setup for semantically irrelevant background facts
+
real authoritative transitions for history relevant to the claim
```

If the path by which a fact arose can affect legality, identity, information, ordering, RNG, history-sensitive behavior, or the expected result, that history must be constructed through authoritative transitions.

Test helpers must not invisibly manufacture semantic history such as:

- cast history;
- damage-dealt-this-turn history;
- reveal/private-look/knowledge history;
- opaque identity retirement/reidentification history;
- priority-pass history;
- Commander cast-count history;
- RNG-consumption history.

## Requirement → Capability → Interaction authority

Recommended conceptual chain:

```text
source/card/rules fact
    ↓
reviewed Requirement
    ↓
reviewed Requirement → exact Capability-version mapping
    ↓
capability dependency closure
    ↓
applicable semantic seams
    ↓
POTENTIAL_SEAM
    ↓ review
REVIEWED_OBLIGATION
    ↓ executable evidence
SATISFIED_EVIDENCE
```

Authority split:

```text
manafold-census
= extraction / normalization / proposals / statistics / potential-seam generation

Manafold
= reviewed mapping / dependency closure / obligations / executable evidence /
  support and certification consequences
```

Census may mirror reviewed Manafold identities but must not become a competing semantic authority.

## Dependency is not interaction

Capability dependencies answer:

> What support must exist?

Interaction obligations answer:

> Which compositions must be proven together?

Do not encode interaction coverage by pretending dependency edges prove composition.

## Risk-scaled evidence policy

```text
LOW
  isolated exact cases
  + relevant pairwise seams
  + ordinary rejection/parity where applicable

MEDIUM
  isolated + pairwise
  + selected ordering / three-way cases
  + decision/information evidence
  + property or bounded evidence where useful

HIGH
  explicit multi-rule pipelines
  + ordering-sensitive evidence
  + soundness/completeness where player choice exists
  + information safety
  + checkpoint/replay/fork evidence
  + properties / bounded exploration / semantic campaigns / mutants where justified
```

Pairwise success never implies arbitrary composition correctness.

Higher-order risk domains include replacement/prevention, triggers, SBA fixed points, continuous effects/layers/dependencies, copy, costs/payments, combat, zone/incarnation/LKI, hidden-information decisions, and priority/stack/forced progress.

## Evidence-family vocabulary

Useful organization families remain:

```text
ZONE
DAMAGE
RESOLUTION
COST
CONTINUOUS / COPY
```

These are evidence/checklist families only. T0 must not implement their semantic pipelines.

## Decision soundness and completeness

Preserve both:

```text
Soundness:
  every offered complete choice is legal

Completeness:
  every legal player-controlled choice in scope is representable
```

Completeness includes combinations, assignments, orderings, target sets, modes, payment choices, replacement choices, trigger ordering, combat assignments, and multi-stage continuation paths where applicable.

No test driver may silently choose a first/default/random candidate or implicit pass.

## Information safety

Real M3 conformance cases should often prove both:

```text
authoritative semantic correctness
+
perspective safety
```

Trusted evidence may inspect full authoritative state. Player/model-facing evidence must remain limited to player-authorized observations, retained information state, visible decisions, observed events, PlayerStep, sanitized errors, and opaque identities.

Useful assertion families include hidden-variation byte noninterference, exact knowledge acquisition/retention/invalidation, correct opaque identity lifecycle, and no trusted ID/seed/RNG/checkpoint leakage.

## Replay / checkpoint / fork

Parity means equivalent future semantics under the accepted anchor contract, not byte-identical complete recorder history where restore/fork intentionally begins a new anchored segment.

At equivalent anchors and identical future inputs, compare semantic state, digests, events, delta, next Decision, player products, status, and required replay semantics.

## Property testing

Property testing is recommended only when a concrete M3 property benefits from generation/shrinking.

Strong candidates include:

- rejected action leaves the full fingerprint unchanged;
- accepted delta reconstructs the exact result;
- irrelevant insertion order does not alter canonical semantic output;
- zone transition identity/LKI invariants;
- checkpoint/restore future-semantic parity;
- hidden variation cannot affect unauthorized output;
- offered action generation remains sound.

The research found `proptest 1.11.0` compatible with the pinned Rust 1.85.1 toolchain, but it is **not a T0 dependency requirement**.

## Fuzzing, bounded exploration, mutation, differential testing

Raw-byte fuzzing and stateful semantic campaigns are separate evidence classes.

Stateful semantic campaigns should drive real Decisions under deterministic semantic budgets, occasionally probing rejection, checkpoint/restore, fork, and replay. They belong after enough real semantics exist (T3 direction), not in T0.

Bounded exhaustive exploration is recommended for small finite domains with genuinely independent simpler oracles, such as priority automata, small replacement/trigger ordering, targets, payments, and bounded combat.

Mutation testing should use a small number of meaningful semantic fault models. A surviving valid mutant is an `EVIDENCE_GAP_CANDIDATE`, not automatically a production bug.

External-engine differential testing remains **signal only**, never semantic authority.

## Staged recommendation

```text
M3.T0
thin internal conformance facade

M3.T1 / S1
first real separately authorized bounded capability + exact evidence

M3.T2
first reviewed multi-capability interaction obligation + evidence

M3.T3
deterministic semantic campaigns and failure promotion
```

The first meaningful real-Magic Rust semantic smoke should be added **with S1**, not before S1 merely to satisfy a checklist label.

## Standard benchmark stress input

The reviewed Standard research was used only as a non-normative stress input:

```text
pair_unique_cards = 26
direct_requirements = 93
advisory_candidate_capabilities = 92
potential_seams = 40
high_priority_interactions = 29
reviewed_obligations = 0
satisfied_interaction_evidence = 0
```

This demonstrates why T0 must remain generic. A design requiring one TestKit helper per candidate capability has already failed.

The benchmark suggests reusable foundations around identity/zone/LKI, turn/priority/forced progress, Decision composition, casting/cost/payment, target legality, event/replacement/trigger/SBA pipelines, damage/history, continuous/copy, combat, information safety, and replay/checkpoint/fork. It does **not** make all of those M3 exit requirements.

## M3 Entry Decision inputs produced by #129

The research is considered complete input for the later M3 Entry Decision:

```text
recommended T0 scope = thin internal conformance facade
scenario policy = structural background + real rule-relevant history
interaction lifecycle = POTENTIAL_SEAM → REVIEWED_OBLIGATION → SATISFIED_EVIDENCE
evidence model = version-bound, risk-scaled, multi-class
legal-action proof = independent oracle vs real production exploration
information safety = capability evidence concern, not later-only audit
replay parity = semantic continuity from accepted anchor
property testing = concrete-value driven, not framework-first
semantic campaigns = T3 direction
bounded exploration = selected finite domains only
mutation = targeted semantic fault models
differential = signal only
S1 = deliberately NOT frozen by Issue #129
```

## Final research status

```text
ISSUE_129_RESEARCH_REVIEW = APPROVE
CURRENT_CONFORMANCE_FOUNDATION_REVIEWED = YES
ORIGINAL_FINDINGS_REEVALUATED = YES
M3_T0_RECOMMENDATION = INTERNAL_THIN_CONFORMANCE_FACADE
TESTKIT_IMPLEMENTATION_RECOMMENDED = YES
TESTKIT_IMPLEMENTED = NO
RULES_CASE_PUBLIC_SCHEMA_FREEZE = NO
FOUNDATION_DEFECT_DISCOVERED = NO
FOUNDATION_READY_FOR_M3 = YES
M3_STARTED = NO
M3_AUTHORIZED = NO
M3_ENTRY_DECISION_INPUT_READY = YES
```

The canonical live execution/scope decision is not this snapshot; it is the separately reviewed M3 entry tracker and authoritative repository contracts.