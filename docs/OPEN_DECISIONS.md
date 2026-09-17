# Open Decisions

**Status:** blocking and resolved decision register; current governance audit snapshot

**Last reviewed:** 2026-09-15

**Audit base:** `origin/master` `b4029d287a5412678cc5088ddc6cb6af063c8e7a`

This register was reviewed row-by-row against the current normative documents,
accepted ADRs, the foundation-freeze evidence, and Issues #178, #163, #129,
and #162. `M3 entry relevance` is an audit classification, not a new
decision-status vocabulary. `DOWNSTREAM_DEADLINE` means that the decision is
intentionally retained for a later milestone; it is not an M3 entry blocker.

| ID | Status | Decision | Deadline | Safe default / current resolution | M3 entry relevance | Evidence |
|---|---|---|---|---|---|---|
| OD-001 | open | Public name and crate namespace | first public release | working name; publish nothing | DOWNSTREAM_DEADLINE | `README.md`; no public-release naming decision |
| OD-002 | resolved | Code/docs license | public repo or contribution | Apache License 2.0; ADR 0034 | NOT_M3_ENTRY_BLOCKER | [`ADR 0034`](adr/0034-apache-license-2.0.md) |
| OD-003 | open | Exact Deck A/B manifests | M4 benchmark/content/bundle planning or first exact deck benchmark freeze | no exact deck support claim, no benchmark freeze, and no card milestone inferred; researched deck is advisory input only | DOWNSTREAM_DEADLINE | [`ROADMAP.md`](ROADMAP.md) M3/M4 boundary; [`CERTIFICATION.md`](cards/CERTIFICATION.md); Issue [#129](https://github.com/chrismaghuhn/Manafold/issues/129) research disposition |
| OD-004 | resolved | Comprehensive Rules snapshot | first real M3 rules case | bind each case to the exact snapshot ID and rule citations; the snapshot alone proves no rule or capability support | NOT_M3_ENTRY_BLOCKER | [`ADR 0051`](adr/0051-comprehensive-rules-snapshot-authority.md); [`Comprehensive Rules snapshot identity`](rules/COMPREHENSIVE_RULES_ARTIFACT_RESEARCH_2026-09-15.md) |
| OD-005 | open | Commander policy and banlist snapshots | first format reset | reject configuration lacking separately pinned format and banlist identities | DOWNSTREAM_DEADLINE | [`FORMAT_MODULES.md`](FORMAT_MODULES.md); [`ADR 0026`](adr/0026-format-state-is-authoritative-state.md); Commander remains M3-deferred |
| OD-006 | open | Oracle/card source and distribution basis | card import | distribute no bulk card data; do not claim card support without source and bundle closure | DOWNSTREAM_DEADLINE | [`ADDING_CARDS.md`](cards/ADDING_CARDS.md); [`CERTIFICATION.md`](cards/CERTIFICATION.md) |
| OD-007 | open | Reference hardware and numerical budgets | performance acceptance | collect metrics only; no performance or throughput claim without pinned hardware and thresholds | DOWNSTREAM_DEADLINE | Issue [#176](https://github.com/chrismaghuhn/Manafold/issues/176) is downstream and non-blocking for M3; the boundary is carried into Issue [#178](https://github.com/chrismaghuhn/Manafold/issues/178); [`ACCEPTANCE_GATES.md`](contracts/ACCEPTANCE_GATES.md) |
| OD-008 | resolved | RNG algorithm and stream derivation | first deterministic reset | `mtgml.rng.v1`: counter-addressed HMAC-SHA-256, 256-bit root seed, typed streams, project-owned bounded sampling and shuffle; ADR 0035 | NOT_M3_ENTRY_BLOCKER | [`ADR 0035`](adr/0035-deterministic-hmac-sha256-counter-rng.md) |
| OD-009 | open | Python/native transport | M5 | M2's temporary non-published subprocess adapter is parity infrastructure only; no production transport decision | DOWNSTREAM_DEADLINE | [`ML_ENVIRONMENT.md`](ML_ENVIRONMENT.md); [`ACCEPTANCE_GATES.md`](contracts/ACCEPTANCE_GATES.md) |
| OD-010 | resolved | Initial canonical wire encoding | M0.1 | canonical UTF-8 JSON; ADR 0017 | NOT_M3_ENTRY_BLOCKER | [`ADR 0017`](adr/0017-canonical-wire-codec.md) |
| OD-011 | open | Semantic action-key and trajectory encoding | first dataset | M2 Decision V2 omits mandatory semantic keys; no dataset publication or stable trajectory meaning | DOWNSTREAM_DEADLINE | [`DECISION_PROTOCOL.md`](DECISION_PROTOCOL.md); [`ML_TRAJECTORIES.md`](ML_TRAJECTORIES.md) |
| OD-012 | partial | Native executor policy/API | first escape hatch | quarantine policy remains accepted; certified bundles reject executors until the policy and executable sandbox contract are complete | DOWNSTREAM_DEADLINE | [`NATIVE_EXECUTOR_POLICY.md`](cards/NATIVE_EXECUTOR_POLICY.md); [`ADR 0024`](adr/0024-native-executor-quarantine.md) |
| OD-013 | open | Loop and shortcut policy | loop-capable primitive | capability unsupported; no loop-capable M3 scope is selected, and any future reachable loop requirement must be reviewed explicitly | NOT_M3_ENTRY_BLOCKER | [`ROADMAP.md`](ROADMAP.md) M3 risk map and bounded exclusions |
| OD-014 | open | Multiplayer utility semantics | multiplayer entry | two-player only; no multiplayer utility or semantics are inferred from M3 planning | DOWNSTREAM_DEADLINE | [`ROADMAP.md`](ROADMAP.md); Issue [#163](https://github.com/chrismaghuhn/Manafold/issues/163) planning boundary |
| OD-015 | partial | API stability and deprecation | first external consumer | M2 closure does not promote APIs; keep Rust/Python/M2 surfaces experimental or provisional unless a separate lifecycle decision registers them, with no external compatibility promise | NOT_M3_ENTRY_BLOCKER | [`API_LIFECYCLE.md`](maintenance/API_LIFECYCLE.md); Issue [#162](https://github.com/chrismaghuhn/Manafold/issues/162) and PR #179 are structural only |
| OD-016 | partial | Toolchain/build image and lockfile | public release | current verification uses pinned Python 3.13.15, Rust 1.85.1, Cargo.lock, and CI evidence; no Level 3 public-release reproducibility strategy is selected | DOWNSTREAM_DEADLINE | [`TOOLCHAIN_POLICY.md`](maintenance/TOOLCHAIN_POLICY.md); merged PR #177 freeze checks |
| OD-017 | resolved | Persisted digest algorithm and canonical state codec version | first persisted checkpoint | unkeyed SHA-256, versioned digest envelope, `mtgml.canonical-cbor.v1`, and detached persisted DTOs; ADR 0038 | NOT_M3_ENTRY_BLOCKER | [`ADR 0038`](adr/0038-persisted-semantic-digests-and-canonical-state-codec.md); [`STATE_HASHING.md`](STATE_HASHING.md) |
| OD-018 | resolved | Capability key grammar/lifecycle | M0.2 | accepted capability model and lifecycle; ADR 0022 | NOT_M3_ENTRY_BLOCKER | [`ADR 0022`](adr/0022-versioned-capability-registry-and-bundle-certification.md); [`CAPABILITY_MODEL.md`](cards/CAPABILITY_MODEL.md) |
| OD-019 | resolved | Exact format-module hook interface | initial M3 boundary | initial M3 is format-neutral with `FormatState::None`; no generic or dynamic hook API is required or frozen, and future typed seams require concrete reviewed evidence | NOT_M3_ENTRY_BLOCKER | [`ADR 0052`](adr/0052-initial-m3-format-neutral-boundary.md); [`FORMAT_MODULES.md`](FORMAT_MODULES.md) |
| OD-020 | open | Search/determinization capability boundary | first search integration | no policy access to full-state forks; no search or determinization work in initial M3 | DOWNSTREAM_DEADLINE | [`ML_ENVIRONMENT.md`](ML_ENVIRONMENT.md); [`ROADMAP.md`](ROADMAP.md) M5/later boundary |
| OD-021 | open | Certification artifact signing/attestation | first public certified bundle | checksums only until the signing/attestation decision and evidence are accepted | DOWNSTREAM_DEADLINE | [`CERTIFICATION.md`](cards/CERTIFICATION.md); [`RELEASE_PROCESS.md`](maintenance/RELEASE_PROCESS.md) |

Resolved rows remain as history. Every resolution requires an ADR describing
choice, alternatives, compatibility, evidence, risks, and the milestone
unblocked. No row remains `ALREADY_RESOLVED_BUT_REGISTER_STALE`: OD-004 and
OD-019 were stale open rows and are now resolved by ADRs 0051 and 0052.

## M3 entry blocker conclusion

Before this governance cleanup, the open-decision blockers were:

```text
M3_ENTRY_BLOCKERS_FROM_OPEN_DECISIONS_BEFORE_THIS_TASK:
OD-004
OD-019

OD-003 = MOVED_TO_M4 / NOT_M3_BLOCKING
```

After the complete audit and the two narrow ADR decisions:

```text
OTHER_OPEN_DECISION_M3_ENTRY_BLOCKERS = 0
UNRESOLVED_OPEN_DECISION_M3_BLOCKERS = 0
```

This conclusion does not approve the M3 Entry Decision, select M3.S1,
freeze capability closure, or authorize M3. Those remain separate Issue #178
gates.

Accepted M2-local ADRs 0039/0040 refine the decision/information architecture
for M2.B; they do not resolve OD-009, OD-011, or unrelated later decisions.
