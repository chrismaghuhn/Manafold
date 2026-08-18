# M0.2 Design Lock Matrix

**Status:** normative snapshot of what is fixed versus deferred

| Surface | M0.2 status | What is locked | What remains open |
|---|---|---|---|
| Project boundary | accepted | headless, ML-native, Rust rules / Python ML, no Argentum dependency | public name/license |
| Trust APIs | accepted | kernel/controller/player separation; perspective-bound endpoint | transport implementation |
| Engine state | accepted model | complete checkpointable components; no hidden semantic state | efficient internal representation |
| Identity | accepted | definition/physical/incarnation/opaque IDs separated | detailed lifecycle for future mechanics |
| Transition | accepted model | atomic validation/commit/rejection; state/events/delta/status together | pinned Magic phase ordering in M3 |
| Decision protocol | provisional-public | every player choice explicit; exact bindings; nonmutation on rejection | semantic-key encoding and full decision inventory implementation |
| Information model | accepted | knowledge state and opaque mappings checkpointable; noninterference | optimized projection representation |
| Wire contracts | provisional-public | canonical v1 player/replay DTOs and fixtures | first external-consumer freeze |
| Error model | accepted taxonomy | player-safe closed codes; trusted errors separate | complete production enum implementation |
| Format modules | accepted boundary | format state inside EngineState; pure deterministic hooks | exact Commander snapshot/semantics |
| Digests | accepted domains | full/public/info/observation/candidate/content/replay separated | final persisted hash algorithm/version |
| Concurrency | accepted | one total semantic order per environment; parallel environments allowed | optimized worker/actor implementation |
| Card IR | experimental | typed/inspectable/serializable direction | concrete variants after M2.5 census |
| Capability model | accepted process | versioned registry, recursive closure, evidence lifecycle | first real registry entries |
| Native executors | prohibited by default | no direct state mutation/I/O/hidden choices | executable API and approval ADR |
| ML trajectories | provisional | one step per player choice; rules-free rewards; no privileged data | v1 wire schema/action keys |
| Performance | measurement contract | required metrics and evidence identity | hardware and numerical budgets |
| V1 content | blocked | exactly two fixed 1v1 Commander decks | deck manifests and source snapshots |

A later implementation may choose data structures and algorithms freely only when it preserves every locked semantic property and proof obligation.
