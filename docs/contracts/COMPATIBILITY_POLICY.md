# Compatibility Policy

**Status:** accepted baseline

Version independently:

- Rust API;
- transition/state/checkpoint codec;
- decisions and semantic keys;
- observations, information state, and observed events;
- errors;
- replay;
- capability registry and bundle/certification manifests;
- Card IR;
- Python API;
- ML trajectories;
- digest domains and canonicalization.

Classify changes as:

- patch-compatible;
- reader-compatible;
- migration-required;
- semantic break.

Unknown required fields or variants fail closed. Optional extensions are namespaced/versioned. Enum/key values never change meaning in place. Migrations create new artifacts with source provenance and never overwrite old data.

Experimental surfaces may break before freeze, but still increment versions, update fixtures, and document impact. Frozen public surfaces follow [`../maintenance/API_LIFECYCLE.md`](../maintenance/API_LIFECYCLE.md).
