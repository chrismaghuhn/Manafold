# Open Decisions

**Status:** blocking and resolved decision register  
**Last reviewed:** 2026-08-20

| ID | Status | Decision | Deadline | Safe default / current resolution |
|---|---|---|---|---|
| OD-001 | open | Public name and crate namespace | first public release | working name; publish nothing |
| OD-002 | resolved | Code/docs license | public repo or contribution | Apache License 2.0; ADR 0034 |
| OD-003 | open | Exact Deck A/B manifests | M2.5 | no card milestone |
| OD-004 | open | Comprehensive Rules snapshot | first rules case | no certified rule |
| OD-005 | open | Commander policy and banlist snapshots | first format reset | reject config |
| OD-006 | open | Oracle/card source and distribution basis | card import | distribute no bulk data |
| OD-007 | open | Reference hardware and numerical budgets | performance acceptance | metrics only |
| OD-008 | resolved | RNG algorithm and stream derivation | first deterministic reset | `mtgml.rng.v1`: counter-addressed HMAC-SHA-256, 256-bit root seed, typed streams, project-owned bounded sampling and shuffle; ADR 0035 |
| OD-009 | open | Python/native transport | M5 | protocols only |
| OD-010 | resolved | Initial canonical wire encoding | M0.1 | canonical UTF-8 JSON; ADR 0017 |
| OD-011 | open | Semantic action-key and trajectory encoding | first dataset | dataset publication blocked |
| OD-012 | partial | Native executor policy/API | first escape hatch | quarantine policy accepted; certified bundles reject executors |
| OD-013 | open | Loop and shortcut policy | loop-capable primitive | capability unsupported |
| OD-014 | open | Multiplayer utility semantics | multiplayer entry | two-player only |
| OD-015 | partial | API stability and deprecation | first external consumer | lifecycle classes accepted; APIs not frozen |
| OD-016 | partial | Toolchain/build image and lockfile | V0.2.2 freeze/release | Python 3.13.5 and Rust 1.85.1 pinned; Cargo.lock, native clean build, and public-release transitive/hash lock or build image still required |
| OD-017 | resolved | Persisted digest algorithm and canonical state codec version | first persisted checkpoint | unkeyed SHA-256 + versioned digest envelope + `mtgml.canonical-cbor.v1` + detached persisted DTOs; ADR 0038 |
| OD-018 | resolved | Capability key grammar/lifecycle | M0.2 | capability model and ADR 0022 |
| OD-019 | open | Exact format-module hook interface | M3 | compile-time Commander only; no hidden state |
| OD-020 | open | Search/determinization capability boundary | first search integration | no policy access to full-state forks |
| OD-021 | open | Certification artifact signing/attestation | first public certified bundle | checksums only |

Resolved rows remain as history. Every resolution requires an ADR describing choice, alternatives, compatibility, evidence, risks, and milestone unblocked.
