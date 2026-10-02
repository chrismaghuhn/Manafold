# Manafold

## Current status

- **Foundation closure/freeze:** `COMPLETE` (`FINAL_FOUNDATION_CLOSURE = PASS`, `PRE_M3_REMEDIATION_FREEZE = PASS`, `FOUNDATION_READY_FOR_M3 = YES`)
- **Core modularization:** Issue #162 `COMPLETE`, merged by PR #179; the refactor was semantic-neutral and did not change public, wire, schema, digest, replay, or rules contracts
- **M2.5 scope work:** `NOT_CLAIMED` / `NOT_FROZEN`; the abandoned census and research machinery remains historical Git evidence, not active engine scope
- **Current status:** M3 is complete and final acceptance passed; M4.2 is complete for the bounded Mountain/Plains executable slice under `basic-land@1.0.0`, after Phase 13 activation and post-merge exact-master verification. M4 remains `IN_PROGRESS`; M4.3 and M4.4 production implementation have not started.
- **Pre-M3 governance cleanup:** `COMPLETE`; the accepted M3 Entry Decision and its historical authorization are preserved, with the hardened scope accepted by PR #184
- **M3 authorization:** `AUTHORIZED` at `ea668c47ef1361b3d989fd32b8f3cfd4751b1e79`; the authorized task at that head was `M3.P0_STATE_IDENTITY_CUT`
- **M3 milestone execution:** `STARTED` — P0 infrastructure is merged and frozen
- **P0:** `COMPLETE / FROZEN` (reviewed head `a7e641a7e6145610c9533187cf6340712f460e44`, merge commit `20dac927027776ef5f0a5b389a27d4a05eefb180`)
- **M3.T0:** `COMPLETE / FROZEN` (closure review head `b403edefcabf7b304c0fa5f6816d22ac8aca477b`, 10/10 frozen exit criteria PASS, 0 BLOCKER / 0 MAJOR; status-sync merge `b9c5f2be97b8fc1f31d648d58f890de78f0a035c`; freeze executed and tracked in Issue #178)
- **M3.S1:** `COMPLETE / COVERED / NOT CERTIFIED` (`rules/turn-structure@0.1.0`; S1 authorization head `587016574e4e8f9f797a713877f8caf1c5143cfb`; covered for the bounded scope, certification is not claimed)
- **M3 semantic implementation:** the eleven bounded Foundation V2 capabilities were implemented on the old rules kernel, which was removed on 2026-09-30 so that one runtime remains; `cards/capabilities/registry.json` lists what the native turn progression covers; none is certified
- **M3.S2:** its selected Battlefield → owner Graveyard and owner Library-top → owner Hand profiles were integrated with combat SBA and ordinary Draw replay on the old rules kernel; the native turn progression keeps Library-top → owner Hand (draw) and adds owner Hand → owner Graveyard (discard)
- **PR #208:** `MERGED`; `S2_EXACT_HEAD_VERIFICATION = PASS`
- **S2 authoritative replay:** the Block 8 exact Foundation V2 integration exercised both SBA-to-graveyard and Draw-to-Hand incarnation transitions through Replay V6 on the removed old runtime; acceptance-time evidence is preserved in `docs/reviews/m3-final-closure-2026-09-25.md`
- **M3 Pre-T0 hardening:** `COMPLETE / ACCEPTED` (`ADR 0054 = ACCEPTED`, `FOUNDATION_V2 = ACCEPTED_HARDENED_M3_SCOPE`)
- **M3 plan status:** `ACCEPTED`
- **Task 14:** `COMPLETE` — `S1_EXACT_HEAD_VERIFICATION = PASS`
- **S3.P0:** `COMPLETE / FROZEN` (reviewed head `0cd24d1f2cb4183c19fb04ce0c3c827148313a3b`, PR #210, merge commit `ffc433985f41e6e2980df23a103b5e2358527ea3`)
- **S3.0:** `COMPLETE / FROZEN` (reviewed head `aa28f9753225dca0e58e33b0d0356a1cc560aae3`, PR #211, merge commit `66f3b713787cad89674257f6e0b6448b9fd568f9`)
- **Magic rules-flow inventory:** `REVIEWED / FROZEN_PLANNING_INPUT` at `8a440645735fcb17be9470c7a53969860b1d4fad`; M3 has 8 major semantic blocks and all 11 Foundation capabilities accounted for
- **PR #213:** `MERGED` at `60b6ee7957032a36371ceac89c3a1e4f886d200c`; reviewed head `fb5830e0354eda14a12641d4166214d63eace4d7` is contained in `master`
- **PR #216:** `MERGED` at `5e474c763a1a67b14536fe5f70824897320e864b`; Combat Phase + Declare Attackers exact-head review passed
- **M3 Block 1:** bounded S3.A is accepted / merged
- **M3 Block 2:** Basic Priority + Reference response integration is accepted / merged
- **M3 Block 3:** Draw + S2 replay/interaction merged by PR #214 at `0a36290`; exact-head review passed
- **M3 Block 4:** Combat Phase + Declare Attackers merged by PR #216 at `5e474c763a1a67b14536fe5f70824897320e864b`; exact-head code review passed. Its capability lifecycles were `specified` when it merged; the current ones are in `cards/capabilities/registry.json`.
- **M3 Block 5:** Declare Blockers merged in PR #217; `FINAL_ACCEPTANCE_PASS` recorded at the accepted exact head. The lifecycle of the affected capabilities was `specified` when it merged; `rules/declare-blockers` is `covered` since the creature blocks and deaths were implemented.
- **M3 Block 6:** Damage/Life, Combat Damage, and post-damage SBA merged in PR #218 at `7060a1212eeb59d511265242be3c720b0fc1cc8a`; exact-head review passed. Its capability lifecycles were `specified` when it merged; `rules/combat-damage`, `rules/damage-and-life` and `rules/state-based-actions-combat` are `covered` now.
- **M3 Block 7:** Cleanup Reset + complete bounded turn merged and accepted in PR #219 at `c7cafa0356508164988fb92b0fcedc24bccbbdae`.
- **M3 Block 8:** cumulative final closure accepted by PR #220; exact Foundation V2 turn integration reaches P2 Draw and its next visible priority Decision.
- **M3 hardening acceptance:** PR #184 merged and accepted ADR 0054/Foundation V2; T0 was reauthorized under Issue #178, implemented by merged PRs #189/#190/#191, and finalized as COMPLETE / FROZEN
- **Capability lifecycle:** 16 capabilities are `covered` and 3 `specified` (`cards/capabilities/registry.json`); `0` are certified.
- **Current boundary:** `M3 = COMPLETE`; `M3_FINAL_ACCEPTANCE = PASS`; `M4 = IN_PROGRESS`; the executable real-card slice is Mountain + Plains under `basic-land@1.0.0` and Savannah Lions + Gray Ogre + Hill Giant under `vanilla-creature@1.0.0`; no broader certification, card/deck/format/Commander/playability support is claimed.
- **Project type:** independent greenfield MTG/ML rules and simulation engine
- **Playable engine:** no
- **Production turn loop:** the V8 runtime plays complete two-player turns with basic lands and vanilla creatures (untap, upkeep, draw, main phases, combat, end step, cleanup with discard to hand size, turn change). A player casts a creature card at sorcery speed through a one-item stack, choosing how to pay when more than one payment exists, and it resolves onto the battlefield; a creature that its controller has controlled since their turn began can attack; the defending player declares blocks, one untapped creature at a time (a creature blocks one attacker or none, and an attacker may be blocked by several creatures); unblocked attackers deal combat damage to the defending player, a blocked attacker and its blocker deal damage to each other, and an attacker blocked by two or more creatures has its controller divide its damage among them, one blocker at a time; damage dealt to a creature stays marked until the cleanup step removes it; a creature with lethal damage dies and goes to its owner's graveyard, and an owner who has two or more cards die together orders them, unless a player loses in the same batch, which ends the game without asking anyone; a player at 0 life or less loses, as does a player who draws from an empty library, and the game ends. The random smoke games still pit a creature deck against a land deck, whose player has no creatures, so none of them blocks. Neither player observes blocks or marked damage yet; both see a creature die. The only abilities are the basic lands' mana abilities, the only spells are creature spells without rules text, and no keyword exists. Proven through the production player endpoints (`crates/mtgml-environment/tests/production_turn.rs`, `creature_game.rs`, and the random smoke games in `random_smoke.rs`, which include a creature deck against a land deck); games start from deck lists (see Game start).
- **Game start:** a game starts from two deck lists as CR 103 prescribes for the first game of a two-player match: a random player chooses who goes first, libraries are shuffled, players start at 20 life with seven cards, players may take London mulligans, and the starting player's first turn has no draw step (`crates/mtgml-environment/tests/game_start.rs`).
- **Real Magic semantics:** the native turn progression covers turn structure, priority, draw, casting a creature spell and its resolution on a one-item stack, summoning sickness, the combat phase with attackers and blockers declared and combat damage to players and to creatures (an attacker blocked by two or more creatures has its controller divide its damage among them), damage and life, combat state-based actions (a player at 0 life or less loses, a creature with lethal damage dies), cleanup with the removal of marked damage, and zone incarnation. This does not claim arbitrary Magic support, cards, decks, formats, or playability.
- **Bounded executable real-card support:** Mountain + Plains under `basic-land@1.0.0` and Savannah Lions + Gray Ogre + Hill Giant under `vanilla-creature@1.0.0`; no broader card/deck support is claimed.

**Current resumable execution contract after the G0j activation cut:** V8.
The production runtime uses the flat `EngineState`, `StateDelta`,
`FullStateDigest` (`mtgml.full-state-digest.v7`), `EnvironmentCheckpointV8` /
`CheckpointDigestV8`, Replay V8, Decision request V4 / response V3, ObservedEvent
V4, PlayerStep V4, `ObservationEnvelope` (`observation-envelope.v2`),
`PlayerInformationState` / `InformationStateDigest`
(`information-state-envelope.v3`), and `magic-shared-execution-observation.v1`. Its executable admission
is bounded to Mountain and Plains under `basic-land@1.0.0` and Savannah Lions,
Gray Ogre and Hill Giant under `vanilla-creature@1.0.0`. It is the only
runtime, and each format has only its current version; older versions were
removed. M4.2 is `COMPLETE` only for the bounded Mountain/Plains slice; this
does not claim broader card, deck, format, or playability support.

Manafold prioritizes:

```text
correctness
→ determinism
→ information safety
→ decision completeness
→ replayability
→ maintainability
→ performance
→ ML scale
```

M1 established the deterministic synthetic kernel shell: complete state construction, accepted/rejected atomic transitions, exact state/event/delta parity, deterministic RNG/allocators, checkpoint/restore/fork/replay parity, and two bound synthetic player endpoints. M2 subsequently closed the decision and synthetic information-safety foundation under the accepted exact-head evidence above.

## Start here

1. [`PROJECT_CHARTER.md`](PROJECT_CHARTER.md)
2. [`AGENTS.md`](AGENTS.md)
3. [`docs/NORMATIVE_HIERARCHY.md`](docs/NORMATIVE_HIERARCHY.md)
4. [`docs/ROADMAP.md`](docs/ROADMAP.md)
5. [`docs/maintenance/MAINTAINER_PROFILES.md`](docs/maintenance/MAINTAINER_PROFILES.md)
6. [`docs/maintenance/DEVELOPER_SETUP.md`](docs/maintenance/DEVELOPER_SETUP.md)
7. [`docs/contracts/ACCEPTANCE_GATES.md`](docs/contracts/ACCEPTANCE_GATES.md)
8. [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
9. [`docs/DOMAIN_MODEL.md`](docs/DOMAIN_MODEL.md)
10. [`docs/EXECUTION_MODEL.md`](docs/EXECUTION_MODEL.md)
11. [`docs/DECISION_PROTOCOL.md`](docs/DECISION_PROTOCOL.md)
12. [`docs/INFORMATION_MODEL.md`](docs/INFORMATION_MODEL.md)
13. [`docs/ML_ENVIRONMENT.md`](docs/ML_ENVIRONMENT.md)
14. [`docs/STATE_HASHING.md`](docs/STATE_HASHING.md)

The ADR index is [`docs/adr/README.md`](docs/adr/README.md).

Generated verification evidence is external to the reproducible source archive. Historical M1/M2 closure claims come from their recorded exact-head evidence and accepted ADRs; future changes must produce fresh evidence rather than relying on prose status.

The maintainer route is [`docs/maintenance/MAINTAINER_PROFILES.md`](docs/maintenance/MAINTAINER_PROFILES.md), with the durable setup path in [`docs/maintenance/DEVELOPER_SETUP.md`](docs/maintenance/DEVELOPER_SETUP.md). The mandatory PR checks are `PR Fast`, `PR Integration`, and the stable aggregate `manafold-pr-gate`.

## Durable boundaries

```text
Trusted kernel
  complete EngineState, authoritative events, exact StateDelta

Trusted environment controller
  configuration, seed, reset, complete checkpoint, restore, fork,
  authoritative replay, scheduling

Perspective-bound player endpoint
  observation, retained information state, visible decision,
  observed events, submit -> PlayerStep

Rules-free Python/ML
  DTO/client consumption, models, rewards, datasets, experiment policy
  no legality/state/RNG authority
```

No player endpoint can obtain full state, root seed, RNG internals, authoritative events, checkpoints, forks, authoritative replay, trusted IDs, or free-form diagnostics.

## M2 contract boundaries

The accepted M2.A architecture requires:

- separate trusted `DecisionId` and perspective-local `PlayerDecisionIdV1`;
- dense request-local `CandidateIdV1`;
- closed answer variants for choose-one/many/number/order;
- typed serialized continuations inside `EngineState`;
- read-only perspective projection;
- retained knowledge keyed through perspective-local opaque identity;
- opaque identity persistence only while distinguishability persists;
- retirement/new identity after hidden randomization;
- one perspective-local visible event sequence;
- independent bounded soundness/completeness proof;
- paired-state byte noninterference;
- a temporary rules-free Python semantic adapter without resolving production transport;
- one coordinated V3 state/digest/checkpoint/replay identity cut.

`EpisodeStatus` remains environment/PlayerStep semantics, not part of retained information state.

Malformed/noncanonical wire bytes fail before a semantic submission and do not synthesize a PlayerStep.

## Historical identity discipline

M2 must not reinterpret M1 V2 state/checkpoint/replay values.

When the runtime `EngineState` changes:

- V2 full-state production is retired;
- V2 in-memory checkpoint semantics are not kept executable by creating a duplicate legacy state model;
- historical fixtures/domains remain immutable evidence;
- historical replay/read/migration support is explicitly classified.

New V3 persisted semantic digests follow ADR 0038 and the byte-level specification in [`docs/STATE_HASHING.md`](docs/STATE_HASHING.md).

## Support claims

The project uses strict lifecycle language:

```text
Imported -> Parsed -> Implemented -> Covered -> Certified
```

Only a certified locked bundle is a real support claim. Parsed/imported/compiled/implemented artifacts are not automatically supported.

No real cards are added before an explicit reviewed scope and capability closure.

## Local verification

Use:

```bash
just doctor
just check-fast
just check
just check-all
just release-candidate
```

Core direct checks include:

```text
<project-python> scripts/verify_repository.py
<project-python> scripts/check_rust_source_structure.py
<project-python> scripts/check_documentation.py
<project-python> scripts/validate_schemas.py
<project-python> scripts/validate_maintainer_artifacts.py
<project-python> scripts/verify_python_toolchain.py
<project-python> scripts/run_python_tests.py

cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
```

Use the platform-specific `<project-python>` paths in the developer setup
document; the scripts reject a non-pinned Python interpreter. `PASS` is
reported only for commands actually executed successfully. Missing/unavailable
tools are `NOT_RUN` or `BLOCKED`.

## Scope discipline

- no hidden/heuristic completion of player choices;
- unsupported semantics fail closed;
- no rules logic in Python or card generators;
- no real cards before exact V1 deck closure is explicitly reviewed;
- no optimized rollout backend before reference parity and profiling;
- no native card executor in a certified bundle under the current quarantine policy;
- no broad support claim from parsing, compilation, or raw card counts.
