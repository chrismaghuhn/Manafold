# Adding Cards

**Status:** accepted maintainer workflow  
**Stability:** process contract; concrete Card IR remains experimental

## Goal

A normal card whose required capabilities already exist should be mostly declarative content plus focused tests. New general semantics are added once as capabilities, not duplicated per card.

## Current workflow

The current implementation uses `scripts/scaffold_card.py` and the workflow below. The command remains the current maintainer entry point; the `just add-card` family described later is a target UX, not an available interface today.

### 1. Scaffold the work item

```bash
python scripts/scaffold_card.py project/card/example-card "Example Card"
```

This creates a manifest, implementation note, and case directory without copying copyrighted bulk source text.

### 2. Pin source provenance

Record source snapshot, source record ID, normalized-source digest, card-definition identity, faces, and generated/reference objects. Source text may be retrieved from the pinned source during authorized local generation; redistribution follows [`SOURCE_AND_GENERATION_PIPELINE.md`](SOURCE_AND_GENERATION_PIPELINE.md).

### 3. Run or author an IR candidate

Parser/LLM/generator output goes under `cards/generated/` with provenance. It is never executable authority until reviewed and promoted to `cards/definitions/`.

### 4. Declare capability requirements

List all direct requirements, including:

- rules/mechanics;
- decisions and ordering;
- visibility/knowledge behavior;
- format interactions;
- generated token/copy/emblem/named-object definitions;
- any currently unsupported case.

Run:

```bash
python scripts/capability_census.py --bundle cards/bundles/<bundle>/manifest.json
```

### 5. Review the decision and information surface

Every player choice must map to the unified protocol. Every reveal/look/hidden-zone transition must specify knowledge and opaque-ID behavior. No auto-target, auto-mode, auto-order, or random payment is allowed in the authoritative environment.

### 6. Add evidence

At minimum:

- normal resolution case;
- illegal/stale/invalid path where applicable;
- relevant interaction case;
- zone/identity and Last Known Information case where applicable;
- replay/checkpoint roundtrip;
- per-perspective observation/event assertions;
- capability closure check.

### 7. Promote status carefully

```text
Imported -> Parsed -> Implemented -> Covered
```

A card becomes **Certified** only through a certified locked bundle. A single passing card test is not certification.

## Expected effort

When all capabilities exist, simple definitions should require little or no new engine code. The first card exposing a new mechanic is expensive because the reusable capability and evidence are the product; later cards reuse it.

## Target Maintainer UX / Golden Path

The long-term target is a stable, command-oriented golden path for the mechanical parts of card maintenance:

```bash
just add-card "Card Name"
just check-card "Card Name"
just add-capability cap.example
just certify-bundle <bundle>
```

These command spellings describe a future target UX. They are not currently implemented, and this document does not define a stable CLI contract for them. The intended `add-card` flow is:

```text
just add-card "Card Name"
        ↓
resolve the card from a pinned source snapshot
        ↓
pin source provenance
        ↓
normalize source data
        ↓
create or refresh a candidate Card IR scaffold
        ↓
derive / declare direct capability requirements
        ↓
compute recursive capability closure
        ↓
generate focused test / conformance scaffolding
        ↓
report missing capabilities
        ↓
leave the card un-certified until review and evidence complete
```

This is intended to automate deterministic, mechanical maintainer work, not semantic authority. A future helper may look up pinned source records, generate scaffolding and Card IR candidates, determine mechanical capability gaps, generate test skeletons, and produce deterministic reports. It may not declare generated Card IR authoritative, invent unsupported Magic semantics, silently approximate card text, mark a card supported, bypass human review, bypass capability or conformance evidence, or certify an isolated card outside the accepted bundle-certification model.

The following invariants remain explicit:

```text
add-card != supported
generated != authoritative
parsed != supported
implemented != certified
```

### Capability-first scaling

The intended cost model is:

```text
new semantic capability
    = expensive once

later cards using that capability
    = mostly declarative + evidence
```

A card whose complete recursive capability closure is already implemented and appropriately covered should normally require little or no new authoritative engine code. A card exposing missing semantics should produce a capability-gap report rather than generate card-specific kernel logic.

For example, a future report could conceptually look like this:

```text
Card: Example Card

Existing capabilities:
  cap.draw                    available
  cap.triggered_ability       available

Missing capabilities:
  cap.some_new_semantic       unsupported

Result:
  scaffold created
  support claim blocked
```

This example is illustrative only; its fields and statuses are not an implemented wire contract.

### Future batch and deck direction

The same architecture should eventually support workflows such as:

```bash
just add-cards cards.txt
just add-deck deck.txt
```

Their deterministic reports could distinguish cards or definitions that are already covered by existing capabilities, require only new card definitions and tests, or are blocked by missing reusable capabilities. These commands and report shapes are future direction only and are not being implemented or specified as a stable CLI contract here.

## Prohibited patterns

- card-name switches in the core kernel;
- direct arbitrary `EngineState` mutation from a definition;
- embedded network/filesystem/time access;
- unversioned free-form script execution;
- silently approximating unsupported text;
- claiming support from successful parsing or compilation alone.
