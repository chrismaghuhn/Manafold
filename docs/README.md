# Documentation Index

**Status:** accepted documentation index  
**Stability:** informative

The machine-readable classification of binding, process, and informative documents is [`normative-document-register.v1.json`](normative-document-register.v1.json).

## Orientation

- [`../PROJECT_CHARTER.md`](../PROJECT_CHARTER.md)
- [`M0_2_SPECIFICATION.md`](M0_2_SPECIFICATION.md)
- [`M1_1_STATE_FOUNDATION_SPECIFICATION.md`](M1_1_STATE_FOUNDATION_SPECIFICATION.md)
- [`V0_2_2_EXECUTABLE_FREEZE_AND_MAINTAINER_ERGONOMICS.md`](V0_2_2_EXECUTABLE_FREEZE_AND_MAINTAINER_ERGONOMICS.md)
- [`V0_2_1_CONTRACT_CLOSURE.md`](V0_2_1_CONTRACT_CLOSURE.md)
- [`VISION.md`](VISION.md)
- [`SCOPE.md`](SCOPE.md)
- [`ROADMAP.md`](ROADMAP.md)
- [`OPEN_DECISIONS.md`](OPEN_DECISIONS.md)

Current executable status is M1 complete. M2.A architecture contracts are
accepted and the M2.B structural implementation is present; the authoritative
`M2_EXECUTABLE_CONTRACT_AND_VERSION_CUT` is `PASS` on the local clean exact
head. Hosted PR evidence and M2.Final closure remain separate.

## Normative architecture and semantics

- [`NORMATIVE_HIERARCHY.md`](NORMATIVE_HIERARCHY.md)
- [`ARCHITECTURE.md`](ARCHITECTURE.md)
- [`PROJECT_STRUCTURE.md`](PROJECT_STRUCTURE.md)
- [`DOMAIN_MODEL.md`](DOMAIN_MODEL.md)
- [`EXECUTION_MODEL.md`](EXECUTION_MODEL.md)
- [`RULES_SEMANTICS.md`](RULES_SEMANTICS.md)
- [`DECISION_PROTOCOL.md`](DECISION_PROTOCOL.md)
- [`DECISION_INVENTORY.md`](DECISION_INVENTORY.md)
- [`INFORMATION_MODEL.md`](INFORMATION_MODEL.md)
- [`ERROR_MODEL.md`](ERROR_MODEL.md)
- [`FORMAT_MODULES.md`](FORMAT_MODULES.md)
- [`STATE_HASHING.md`](STATE_HASHING.md)
- [`RNG_CONTRACT.md`](RNG_CONTRACT.md)
- [`CONCURRENCY_MODEL.md`](CONCURRENCY_MODEL.md)
- [`REPLAY_AND_DETERMINISM.md`](REPLAY_AND_DETERMINISM.md)
- [`ML_ENVIRONMENT.md`](ML_ENVIRONMENT.md)
- [`ML_TRAJECTORIES.md`](ML_TRAJECTORIES.md)

`STATE_HASHING.md` owns the accepted ADR-0038 byte-level digest-envelope / `mtgml.canonical-cbor.v1` specification for the M2 V3 structural implementation.

## Content and rules maintenance

- [`CARD_IR.md`](CARD_IR.md)
- [`rules/AUTHORITY_POLICY.md`](rules/AUTHORITY_POLICY.md)
- [`rules/ADDING_RULES_AND_MECHANICS.md`](rules/ADDING_RULES_AND_MECHANICS.md)
- [`rules/MECHANIC_LIFECYCLE.md`](rules/MECHANIC_LIFECYCLE.md)
- [`cards/ADDING_CARDS.md`](cards/ADDING_CARDS.md)
- [`cards/CAPABILITY_MODEL.md`](cards/CAPABILITY_MODEL.md)
- [`cards/CERTIFICATION.md`](cards/CERTIFICATION.md)
- [`cards/SOURCE_AND_GENERATION_PIPELINE.md`](cards/SOURCE_AND_GENERATION_PIPELINE.md)
- [`cards/NATIVE_EXECUTOR_POLICY.md`](cards/NATIVE_EXECUTOR_POLICY.md)
- [`cards/CARD_DIRECTORY_LAYOUT.md`](cards/CARD_DIRECTORY_LAYOUT.md)

## Testing, debugging, and performance

- [`TESTING_AND_CONFORMANCE.md`](TESTING_AND_CONFORMANCE.md)
- [`testing/CONFORMANCE_AUTHORING.md`](testing/CONFORMANCE_AUTHORING.md)
- [`testing/NONINTERFERENCE_TESTING.md`](testing/NONINTERFERENCE_TESTING.md)
- [`testing/PROPERTY_AND_FUZZING.md`](testing/PROPERTY_AND_FUZZING.md)
- [`OBSERVABILITY_AND_DEBUGGING.md`](OBSERVABILITY_AND_DEBUGGING.md)
- [`DEBUG_ARCHITECTURE_CONTRACT.md`](DEBUG_ARCHITECTURE_CONTRACT.md)
- [`PERFORMANCE.md`](PERFORMANCE.md)

## Binding contract sheets

- [`contracts/M0_2_DESIGN_LOCK_MATRIX.md`](contracts/M0_2_DESIGN_LOCK_MATRIX.md)
- [`contracts/SEMANTIC_CONTRACT.md`](contracts/SEMANTIC_CONTRACT.md)
- [`contracts/ML_CONTRACT.md`](contracts/ML_CONTRACT.md)
- [`contracts/PLAYER_API_CAPABILITY_MATRIX.md`](contracts/PLAYER_API_CAPABILITY_MATRIX.md)
- [`contracts/ENGINE_STATE_CLOSURE.md`](contracts/ENGINE_STATE_CLOSURE.md)
- [`contracts/WIRE_CONTRACT.md`](contracts/WIRE_CONTRACT.md)
- [`contracts/V1_SCOPE_MATRIX.md`](contracts/V1_SCOPE_MATRIX.md)
- [`contracts/ACCEPTANCE_GATES.md`](contracts/ACCEPTANCE_GATES.md)
- [`contracts/COMPATIBILITY_POLICY.md`](contracts/COMPATIBILITY_POLICY.md)

## Maintainer process

- [`MAINTAINER_PLAYBOOK.md`](MAINTAINER_PLAYBOOK.md)
- [`IMPLEMENTATION_STANDARDS.md`](IMPLEMENTATION_STANDARDS.md)
- [`THREAT_MODEL.md`](THREAT_MODEL.md)
- [`maintenance/API_LIFECYCLE.md`](maintenance/API_LIFECYCLE.md)
- [`maintenance/FREEZE_LEVELS.md`](maintenance/FREEZE_LEVELS.md)
- [`maintenance/OWNERSHIP_MODEL.md`](maintenance/OWNERSHIP_MODEL.md)
- [`maintenance/DEPENDENCY_POLICY.md`](maintenance/DEPENDENCY_POLICY.md)
- [`maintenance/TOOLCHAIN_POLICY.md`](maintenance/TOOLCHAIN_POLICY.md)
- [`maintenance/RELEASE_PROCESS.md`](maintenance/RELEASE_PROCESS.md)
- [`maintenance/SCHEMA_EVOLUTION.md`](maintenance/SCHEMA_EVOLUTION.md)
- [`adr/README.md`](adr/README.md)

Accepted ADRs 0039 and 0040 contain the M2 decision and information/V3 compatibility architecture; accepted ADR 0041 records the post-`M2.Final` capability-oriented semantic-ownership decision; accepted ADR 0042 records the ContextApplicationV2 reviewed-context bridge architecture. ADRs record intent; executable M2 closure remains evidence-driven.
