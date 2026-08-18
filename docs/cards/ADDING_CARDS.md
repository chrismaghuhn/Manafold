# Adding Cards

**Status:** accepted maintainer workflow  
**Stability:** process contract; concrete Card IR remains experimental

## Goal

A normal card whose required capabilities already exist should be mostly declarative content plus focused tests. New general semantics are added once as capabilities, not duplicated per card.

## Standard workflow

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

## Prohibited patterns

- card-name switches in the core kernel;
- direct arbitrary `EngineState` mutation from a definition;
- embedded network/filesystem/time access;
- unversioned free-form script execution;
- silently approximating unsupported text;
- claiming support from successful parsing or compilation alone.
