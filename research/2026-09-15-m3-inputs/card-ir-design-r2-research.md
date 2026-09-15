# Manafold Card IR — Reviewrevision R2 Research Snapshot

**Status:** DESIGN_ONLY / NON-NORMATIVE / NOT AN ACCEPTED CARD-IR SCHEMA

**Source package identity:**

```text
filename = Manafold_Card_IR_Designentwurf_Reviewrevision_R2(2).zip
size = 124471
sha256 = a036e905032d436a930d9567334efebfec2536522d926413d39e3d018b401999
entries = 28
```

The source package is a design/review bundle dated 2026-09-13. Its focus is early continuous-effects evidence and controlled specialization of grouped operations. It is not a production Card IR implementation and does not alter Manafold's accepted Card IR contracts.

## Package contents of interest

The package contains research documents covering:

```text
CARD_IR_DESIGN.md
CONTINUOUS_EFFECTS_EVIDENCE.md
GROUPED_OPERATION_DESIGN.md
NODE_CONTRACTS.md
TEST_MATRIX.md
SOURCE_REGISTER.md
REVIEW_CHANGELOG.md
REVIEW_GATES.md
VALIDATION_REPORT.md
```

It also contains a machine-readable case catalog, detailed proposed case descriptions, six example JSON programs, package-validation tooling, patches against the earlier design package, and the unchanged original design package for provenance.

## Core status

The source package explicitly classifies itself as:

```text
DESIGN_ONLY
```

It includes six unchanged syntactically valid JSON program fragments and 64 proposed domain cases, including 32 newer detailed case descriptions.

Those cases are:

```text
NOT_RUN
```

and proposed expected traces require independent semantic review.

Package validation/checksums prove package integrity only. They do not prove Magic correctness, Card IR type soundness, support, coverage, or certification.

## Useful architectural direction

The design research reinforces Manafold's existing philosophy:

```text
source data
→ normalization
→ parser/generator candidate where useful
→ typed Card IR
→ static validation
→ human review
→ conformance
→ interaction evidence
→ certified bundle
```

Generated/parsed content is never automatically authoritative.

General reusable Magic semantics belong in rules/mechanic capabilities. Card IR should compose those semantics rather than becoming a second rules engine or hiding arbitrary executable logic.

## Continuous-effects emphasis

R2 specifically investigates how Card IR might represent continuous effects early enough that the architecture does not harden around only simple one-shot operations.

The research should be used to challenge later M4 design against real required semantics such as:

- characteristics and modifiers;
- duration/scope;
- source/controller/affected-role distinctions;
- dependency/timestamp concerns where required;
- interaction with copy/layer semantics;
- information-safe representation where characteristics are hidden or perspective-dependent.

This does **not** mean the package freezes a complete layers/copy architecture.

## Grouped-operation emphasis

The package explores controlled specialization for grouped operations rather than forcing every operation into one overly generic AST form.

The direction to preserve is:

- share infrastructure when semantics are truly shared;
- keep distinct semantic obligations typed and reviewable;
- do not use a Turing-complete generic escape hatch to avoid designing real semantics;
- do not duplicate equivalent behavior card-by-card.

## Native/custom execution boundary

This research must not be interpreted as permission to bypass the normal state/event/Decision/information/replay contracts through arbitrary native card executors.

If future real cards expose a missing general semantic primitive, that behavior returns to the reusable capability workflow.

## Relationship to M3

This package is **not** part of the Initial Semantic Foundation merely because it is stored alongside M3 research inputs.

Correct sequencing remains:

```text
M3
freeze/implement/cover reusable Magic semantics

↓

M4
review concrete Card IR against real cards and exact capability requirements
```

The Standard benchmark research is particularly useful later because it can provide real card requirements with which to test whether Card IR remains declarative and whether the semantic capability boundary is correctly drawn.

Do not implement a broad Card IR redesign before the required reusable M3 semantics and reviewed card requirements make the need concrete.

## Evidence boundary

The package reports local package-validation work, but those local review gates are not accepted Manafold gates.

```text
PACKAGE_CHECKSUM_VALIDATION != MANAfold_CONFORMANCE
PROPOSED_CASE != EXECUTED_CASE
JSON_EXAMPLE != ACCEPTED_SCHEMA
DESIGN_COMPLETE != CONTRACT_FROZEN
```

No repository change, card/deck support, or productive implementation was made by the source research task.

## Reviewed preservation status

```text
RESEARCH_ONLY = YES
DESIGN_ONLY = YES
CURRENT_M3_SCOPE_INPUT = INDIRECT_ONLY
M4_CONTENT_ARCHITECTURE_INPUT = YES
CARD_IR_SCHEMA_FROZEN_BY_THIS = NO
CARDS_IMPLEMENTED = NO
BUNDLE_CERTIFIED = NO
```

The package should be revisited after the first real capability/card vertical slices provide concrete evidence about what the IR must express.