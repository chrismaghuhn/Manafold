# Threat Model

**Status:** accepted baseline

## Protected assets

- hidden card/state information;
- root seeds and RNG stream internals;
- authoritative object/ability/event/continuation IDs;
- full checkpoints and replays;
- certified artifact identity and provenance;
- deterministic equivalence of published datasets.

## Trust zones

- trusted kernel/controller/replay/conformance tooling;
- restricted maintainer diagnostics;
- perspective-bound endpoint;
- untrusted model/policy process;
- external source/generation tools;
- future network/service boundary.

## Primary threats

- direct full-state/event/replay access from player APIs;
- transitive type or error-message leaks;
- hidden-state-dependent candidate order/count/semantic keys;
- global event counts or allocator history as side channels;
- root seed/replay leakage into datasets;
- generated/native card code bypassing state/decision boundaries;
- nondeterministic thread/map/time behavior;
- malicious or malformed wire/content artifacts;
- dependency/source substitution and uncertified bundle claims.

## Controls

- capability-separated APIs and closed DTOs;
- explicit player-safe error mapping;
- checkpointed knowledge/opaque mappings;
- paired-state noninterference;
- canonical validated readers/writers;
- fail-closed capability/bundle loading;
- no I/O/time/global RNG in semantic code;
- provenance digests, lockfiles, clean builds, and generated verification;
- native-executor quarantine.

## Out of current scope

Strong remote timing resistance, hostile multi-tenant sandboxing, signed attestations, and online authentication are later deployment concerns. M0.2 prevents deliberate semantic leaks but does not claim a hardened network service.
