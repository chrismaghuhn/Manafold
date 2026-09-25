# M4 Versioned Contract Growth Audit

**Status:** research and architecture audit; no version is allocated by this report.
**Baseline:** 7a26e519a42743d2307f52d99539e1ae01ffe417 (origin/master, fetched 2026-09-25).

## 1. Authority and baseline

`git fetch origin` completed. `origin/master` and the audit worktree HEAD both resolve to `7a26e519a42743d2307f52d99539e1ae01ffe417`; the branch began clean at that commit. Other worktrees were preserved.

Tracker verification: #221 is CLOSED; #222 is OPEN; #224 is MERGED at the baseline SHA. #225 is OPEN with an empty body. This differs from the expected status “M4.2 paused at state/versioning authority boundary”: the repository/tracker does not record that pause. This report treats #222 and current master as authority and records the discrepancy; it does not edit tracker state. #221 final acceptance evidence reports implementation, hosted CI, code review, post-merge exact-master, and final acceptance PASS.

The exact locked R1/W1 compositions in #222 are used without modification. This checkout has no repository-owned exact locked-deck manifest in `cards/decks`; the R1 archive is explicitly research-only. Thus #222’s lists and aliases remain operative, with a source-controlled exact manifest/provenance handoff needed before runtime bundle identity work.

### Authorities consulted

Normative sources inspected: `docs/NORMATIVE_HIERARCHY.md`, `docs/STATE_HASHING.md`, `docs/REPLAY_AND_DETERMINISM.md`, `docs/DOMAIN_MODEL.md`, `docs/EXECUTION_MODEL.md`, `docs/DECISION_PROTOCOL.md`, `docs/INFORMATION_MODEL.md`, `docs/cards/CAPABILITY_MODEL.md`, `docs/contracts/CARD_DEFINITION_CONTRACT.md`, `docs/contracts/ACCEPTANCE_GATES.md`, ADR-0038, ADR-0054, ADR-0055, M4 master and M4.1 spec/evidence, capability registry, relevant Rust crates, and `research/2026-09-24-magic-rules-flow-inventory.md`.

Official Magic authority: Wizards Comprehensive Rules artifact effective 2026-09-25, MagicCompRules 20260925, SHA-256 `8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`. Source index: [Wizards Rules](https://magic.wizards.com/en/rules); artifact: [2026-09-25 Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt). Land/mana clauses used: CR 106.4/106.4b, 116.2a, 117.1d/117.3a, 118.3/118.8/118.9, 202, 302.6, 305.1–305.3/305.6, 400–406, 601.2, 602, 605.1a/605.3/605.3a–b, 608, 701.21, 704. Other card mechanics need focused rule-clause review before implementation.

Oracle source: Scryfall bulk data current 2026-09-25. Oracle Cards timestamp `2026-09-25T09:01:55.126Z`, archive `oracle-cards-20260925090155.jsonl.gz`, SHA-256 `004bb27dfb3f4e41da7ca4ac77808551f54b4bbc152512f2e791546fe95ae709`; Rulings timestamp `2026-09-25T09:00:32.085Z`, archive `rulings-20260925090032.jsonl.gz`, SHA-256 `d9da67fb238ee396956a56c637d33d7c29eb4166376e682bbf5e6442e1f5dea2`. Ordered normalized requested-record extraction: SHA-256 `f07f99115b3bd8dd5f7076da63560a5f01b31777fae5d05fcbea48e2cbde18d5`. Source: [Scryfall bulk data](https://scryfall.com/docs/api/bulk-data), [Oracle records](https://data.scryfall.io/oracle-cards/oracle-cards-20260925090155.jsonl.gz), [Rulings](https://data.scryfall.io/rulings/rulings-20260925090032.jsonl.gz). These are audit-input hashes, not Manafold content identities.

## 2. Horizon map

| Horizon | Required semantic reach | State-growth implication |
|---|---|---|
| H1 / M4.2 | First real slice; selected recommendation: Mountain + Plains basic-land family. | Land-play usage and mana pool are new authoritative state; intrinsic mana abilities must execute. |
| H2 / M4.3 | Full locked R1: 4 each Fanatical Firebrand, Hired Claw, Magebane Lizard, Emberheart Challenger, Razorkin Needlehead, Hearthborn Battler, Ojer Axonil, Nova Hellkite, Burst Lightning, Lightning Strike; 16 Mountain; 4 Rockface Village. | H1 pool must include color and Rockface’s creature-spell restriction. R1 also requires turn history, counters, transformation, warp, triggers, spell/ability payload and damage/replacement semantics. |
| H3 / M4.4 | Full locked W1 from #222: 16 Plains; Ethereal Armor; Spellbook Vendor; Ruin-Lurker Bat; Feather of Flight; Optimistic Scavenger; Shardmage’s Rescue; Sheltered by Ghosts; Seam Rip; Origin of Spider-Man; Skyward Spider; Abandoned Air Temple; Evershrike’s Gift; Dryad Militant. Aliases: A Most Helpful Weaver → Origin of Spider-Man; Wonderweave Aerialist → Skyward Spider. | Attachments, counters, Saga, generated objects, linked exile, temporary effects, ward/payment decisions, graveyard return and replacement semantics. |
| H4 / M4.5 | Cross-deck interactions. | May expose interactions among state families; no extra family pre-authorized here. |
| H5 / M4.6 | Initialization and playable game: starting player, shuffle, opening hands, London mulligan, first-player draw exception, turn/priority, land/mana/cast/target/payment/stack/triggers/combat/cleanup/outcome. | Mulligan requires cross-choice continuation. Pregame provenance belongs in initial identity/control input unless runtime legality requires state. |
| H6 / M4.7–M4.12 | Cumulative zero-unsupported and privacy/decision/replay closure, bundle identity, certification. | Certification metadata is not gameplay state absent explicit contract authority. |

## 3. Current versioned/frozen contract inventory

“Closed” means fixed fields/variants; a semantic addition needs a successor or separately named compatible payload. A Rust type can require a source cut without being a wire version.

| Contract | Current identity/owner | Closure and cut trigger | Historical rule | M4 pressure |
|---|---|---|---|---|
| EngineState + validation | Rust `mtgml-state`; revision/core/combat/foundation_sources/zones/allocators/execution/random/knowledge/perspective_identities/format | Closed validated aggregate; future-legality fact needs typed field/invariant/canonical identity | Never reinterpret | H1 required |
| FullStateDigest | FullStateDigestV5; `full-state-digest-input.v5`, `mtgml.full-state-digest.v5` | Fixed 13-element canonical CBOR; new authoritative state requires successor | V5 exact; detached verification only | H1 required |
| StateDelta, before/after digest, EngineStateParts replacement | Typed StateDelta directly references V5 | Must type-forward with digest successor and full replacement | Historical typed product keeps meaning | H1 coupled type cut |
| EnvironmentCheckpoint | EnvironmentCheckpointV6 | Complete snapshot bound to V5 identity | Old checkpoint unsupported by new runtime; archived runtime only | H1 required |
| CheckpointDigest | CheckpointDigestV6; `environment-checkpoint-digest-input.v6`, `mtgml.checkpoint-digest.v6` | Closed input includes V5 digest and execution identity | Exact historical identity | H1 required |
| Replay manifest/step/container/recorder/schema inventory | Replay V6 and InitialEnvironmentIdentityV6 | Directly type-bind V5/V6 identities and schema IDs | Detached validation/read; archived runtime execution | H1 required |
| Decision request/response | Decision V2, closed CandidateIntent; generic DecisionResponseV2 | New PlayLand intent cannot overload CastSpell/SelectObject; generic response may remain | Exact old request meaning | Request cut H1; response no change likely |
| EngineCandidateBinding | Decision/rules binding | ActivateAbility has ID but no authoritative source/definition/face/ability mapping | Existing gap, not a migration | Typed derivable binding needed H1 |
| Player decision wire / PlayerStep | Request V2 / PlayerStep V2 | Closed action vocabulary; PlayerStep embeds request/actions/events | Old schema immutable | H1 request and PlayerStep cut |
| ObservationEnvelope / ObservationDigest | Envelope V1 and digest of canonical envelope bytes | Codec-ID plus bytes already allows a new named codec | Old codec meanings immutable | No envelope/digest cut |
| Magic observation payload | Named codecs | New public mana/candidate view gets new codec; no reinterpretation | Existing codec IDs fixed | H1 new codec |
| InformationStateDigest | V2 | Includes observation + retained knowledge; no new field if payload carries new view | Exact | No change likely |
| Authoritative events / SemanticDeltaOperation | Closed Rust enums; not separate replay wire versions | Add typed land/mana evidence and later only required event semantics | Preserve variants | H1 enum growth; no replay cut solely for enum |
| ExecutionIdentityV1 | Stable program kind + SemanticContractIdV1 | Values change, schema need not | Exact | No change |
| SemanticContractId/manifest V1 | Existing manifest fields, optional content-contract ID | New IDs/closure can be values | Exact | No change |
| RulesContractId/manifest | Current typed rules manifest | Capability keys/versions are values if schema fits | Exact | No schema cut likely |
| ContentContractIdV1 / CardDefinition envelope | M4.1 fixed envelope/hash | Reserved `profiled` typed seam; body must be fixed closed schema and unprofiled bytes stay exact | V1 stays if seam is honored | No outer cut likely |
| CardSemanticProfile | No executable profile yet | New bounded, typed, canonical, requirement-deriving profile contract | New semantics, no VM | H1 required |
| Capability Registry | V1 | Typed entries/lifecycle values can grow | Old entries exact | No schema cut likely |
| RNG | V1 | H1 has no randomness | Exact | No change |
| FormatState | Optional current state slot | No Standard runtime state; deck/cert identity is not game state | Exact | No H1 change |

## 4. M4.2 basic-land state census

CR 305.1–305.3: land play is a special action from hand to battlefield, not stack use, under active-player/main-phase/priority/empty-stack timing and land-play entitlement. CR 305.6 gives basic land types intrinsic mana abilities. CR 605.3a–b: mana ability resolves immediately/no stack. CR 106.4 empties unspent mana at each step/phase end; 106.4b makes pools public. Definition subtype—not card name—must derive ability.

Current master confirms `GameObject.tapped`, active player/turn position/priority, Hand/Battlefield zones, zone incarnation, AbilityInstanceId allocator and opaque ability identity concepts, and `activate_ability` intent. It lacks ManaPool, land-play-use tracking, and PlayLand intent. Registry has covered basic priority, turn structure, zone incarnation, draw, damage/life and combat/cleanup families; it lacks production land play, intrinsic basic-land mana, mana production/pool/payment, spell casting/cost legality, target legality, noncombat damage, and spell stack payload/resolution. Research placeholders are not implementation evidence.

Land-play use cannot be reconstructed from battlefield: a played land may leave and leave no trace distinguishing the player from one who never played. Add per-player current-turn use state. Locked M4 contains no extra-land effect, so freeze only the bounded normal entitlement and used state/reset; additional land permissions fail closed.

Mana is per player and distinguishes W/U/B/R/G/C; generic is a cost symbol, not a mana type. Include the finite restriction class required by R1 Rockface Village: unrestricted and creature-spell-only. No source provenance, snow, conditional, persistent or arbitrary metadata is justified; reject unsupported classes. Define bounded counts, overflow rejection, restriction spending, deterministic representation and phase/step clearing before implementation.

| Fact | Required classification | Owner/invariant | Evidence/visibility |
|---|---|---|---|
| Basic subtype/intrinsic ability | IMMUTABLE_CONTENT_DATA | Typed definition; no name switch | Public characteristics; no internal IDs |
| Land-play-used | NEW_AUTHORITATIVE_STATE_REQUIRED | Per-player turn-scoped fact, reset at correct boundary; bounded normal entitlement | LandPlayed event/delta; public action availability |
| Hand→Battlefield/incarnation | EXISTING_AUTHORITATIVE_STATE | Existing zones/incarnation | Exact zone transition/object |
| Tapped | EXISTING_AUTHORITATIVE_STATE | `GameObject.tapped` | Tap cost delta |
| Ability authority | DERIVED_STATE_ONLY | Object incarnation + content-scoped Definition + FaceKey + AbilityKey | Opaque request-local candidate |
| Persistent ability registry/allocator | NOT_REQUIRED_FOR_LOCKED_M4 | No registry for static basic ability; reject stale tuple | Never expose trusted IDs |
| Mana pool (six colors + closed restriction) | NEW_AUTHORITATIVE_STATE_REQUIRED | Per-player, bounded nonnegative amounts, restriction preserved | Public by CR 106.4b; mana add/spend/empty evidence |
| Activation workspace | TRANSIENT_RULES_WORKSPACE | Validate source/controller/tapped/ability, produce typed mana | Atomic RulesKernel TransitionProduct |
| Choice | DECISION/CONTINUATION_STATE | Player selects among legal source candidates; no AutoPay | Existing boundary, no continuation for immediate ability |
| Seed/setup | REPLAY_CONTROL_DATA | Initial/replay identity, not duplicate game field | Replay initial identity |

Mountain + Plains share one typed profile parameterized by basic subtype/color and provide evidence for two mana colors. This is a justified tiny family, not convenience. Lightning Strike is materially larger: casting, cost/payment, stack spell payload, target domain/legality, priority, resolution, noncombat damage, graveyard transition and atomic rejection. Most capability roots are absent. H1 recommendation: Mountain + Plains land play, intrinsic tap-to-add mana, mana clearing at phase/step boundary, exact state/event/delta, checkpoint/fork/replay.

## 5. R1 state-growth census

Current Scryfall Oracle identities resolved; this is dependency analysis, not implementation evidence.

| Card / Oracle identity | Material Oracle clauses | Additional state/contract pressure |
|---|---|---|
| Fanatical Firebrand `d36e11f1-6ab3-4273-8114-a8fbbe21c1c3` | Haste; tap+sacrifice for 1 damage to any target | Activated ability cost/source; target; stack/direct damage; life/death/SBA |
| Hired Claw `031cfc7d-8bda-454b-b88a-15d3f5fe47e2` | Attack trigger damages opponent; `{1}{R}` adds +1/+1 counter only if opponent lost life this turn and once/turn | Turn-scoped life-loss; counters; once-use; trigger/target/payment |
| Magebane Lizard `0c38f159-367b-4e4b-94cd-bf1b0721c3a8` | Noncreature spell trigger deals damage equal to player’s noncreature spells this turn; ruling counts before source entered | Per-player spell history; trigger payload; noncombat damage |
| Emberheart Challenger `5b4b21ed-c24e-4038-83fb-bd7f3c3426cd` | Haste/prowess; first own spell/ability target each turn exiles top card and permits play this turn | Temporary modifier; once-target history; exile permission/duration; hidden decision |
| Razorkin Needlehead `a78f981a-bf8a-42a4-b171-d655cc2cc1a2` | First strike on own turn; on opponent draw, damage that player when damaged | Verify existing characteristic support; draw trigger/source and damage |
| Hearthborn Battler `beb02f49-95a6-49bf-b176-4e4b2ab5b651` | Any player’s second spell that turn triggers 2 damage to target opponent | Spell counts; trigger, target and damage |
| Ojer Axonil `d3b7b541-6f05-46c1-8031-c848c4bd4635` | Front trample/replacement for red noncombat damage; dies returns tapped/transformed; back land mana and transform condition based on ≥4 red noncombat damage this turn | Face state; aggregate turn damage; replacement; linked return/tap; DFC profile |
| Nova Hellkite `9635d79e-62ec-4d80-ae37-2793489cc13b` | Flying/haste; ETB damage; Warp delayed exile and later cast permission | Trigger/target; delayed boundary; permission/duration |
| Burst Lightning `ac2086fe-98ee-4280-9c7c-c5d2c6548a8b` | `{R}`, any target, 2 damage; kicker `{4}` for 4 | Stack, target, payment/kicker, resolution/damage |
| Lightning Strike `f34b9bc4-7bfe-47fd-ba23-4eeeb46026eb` | `{1}{R}`, 3 damage to any target | Cast/payment/target/stack/noncombat damage |
| Mountain `a3fb7228-e76b-4e96-a40e-20b5fed75685` | Basic Land—Mountain; intrinsic red mana | H1 |
| Rockface Village `7e103748-3f76-42ce-a063-d0256b2dce2b` | Colorless; restricted red for creature spells; `{R}, {T}` grants eligible creature +1/+0 and haste until EOT at sorcery timing | Restriction in H1; later temporary effect/type filter/activation |

**R1_NEW_AUTHORITATIVE_STATE_UNION:** H1 land/mana; per-player spell counts; opponent-life-loss fact; counters; once/turn use; temporary modifiers/expiry; target-history/use; exile/play permissions; transformed face; per-turn red noncombat damage aggregate; delayed Warp effect; trigger/stack payload and target context. Exact reusable state schemas are UNKNOWN / REQUIRES_FOCUSED_FOLLOWUP. Event history alone is not state when future legality depends on it.

## 6. W1 state-growth census

| Card / identity | Material Oracle clauses | Additional state/contract pressure |
|---|---|---|
| Plains `bc71ebf6-2056-41f7-be35-b2e5c34afa99` | Basic Land—Plains; intrinsic white mana | H1 |
| Ethereal Armor `dbba75f5-2404-4bd5-982b-f6c4effa5316` | Aura; +1/+1 per enchantment controlled; first strike | Attachment; derived characteristics |
| Spellbook Vendor `51dad7d3-f91d-4bd8-aaed-235da42a448b` | Vigilance; optional beginning-combat payment then later target choice creates attached Sorcerer Role | Trigger/reflexive trigger, choice, token, Role attachment/effect |
| Ruin-Lurker Bat `e6f379ad-96b1-46c3-bbc6-a8cb57156a7f` | Flying/lifelink; end-step scry if descended this turn; ruling defines permanent card put in graveyard this turn | Turn history survives card leaving graveyard; scry choice |
| Feather of Flight `c61b05b8-00f1-4371-9856-c65852b4ca02` | Flash Aura, ETB draw, +1/+0 and flying | Attachment, trigger, hidden draw |
| Optimistic Scavenger `0180c67b-c5c8-4a67-9562-d51f9e7ffff5` | Enchantment entry or fully unlocked Room triggers counter on target | Counter/trigger/target; Room reachability unproven, defer Room state |
| Shardmage’s Rescue `406247ee-2b86-4068-b5fa-65cdc83a82f4` | Flash own-creature Aura, +1/+1, hexproof while Aura entered this turn | Attachment; turn-scoped entry condition |
| Sheltered by Ghosts `d13fc657-c6fc-4394-bc70-691050550226` | Aura; ETB exile opposing nonland until Aura leaves; grants +1/+0/lifelink/ward 2 | Attachment, linked exile, grants, ward payment |
| Seam Rip `dfd5939e-a71a-4e16-9b93-752162be4ccd` | ETB exile opposing nonland MV≤2 until source leaves | Link/return relation, target legality, new incarnation |
| Origin of Spider-Man `7fbe9056-190f-40a1-bee9-59ebd6f981f5` | Saga lore I makes Spider token; II counter and target becomes legendary Spider Hero; III double strike EOT, then sacrifice | Lore/chapter state, token, type change, temporary ability and timing; alias provenance |
| Skyward Spider `d6ba76fa-441c-4a74-bfea-8ae2a2c9e391` | Ward 2; flying while modified by Aura/equipment/counter | Counter/attachment queries, derived characteristic, ward; alias provenance |
| Abandoned Air Temple `9575d7ce-f26d-4b90-87a3-6329e9799572` | Enters tapped unless basic controlled; white mana; paid activation adds +1/+1 counters to creatures | H1 mana, ETB condition, counter family |
| Evershrike’s Gift `b5bc4adf-9105-4901-989c-63612314af5c` | Aura flying/+1/+0; graveyard `{1}{W}`, Blight 2 returns to hand at sorcery timing | Attachment; graveyard activation; -1/-1 counters/annihilation |
| Dryad Militant `b8ca5877-9e4c-4b15-8c23-c70f61b01895` | Hybrid cost; instant/sorcery cards that would enter graveyard from anywhere are exiled instead | Hybrid payment; source-derived replacement and zone move |

**W1_NEW_AUTHORITATIVE_STATE_UNION:** typed counters; attachments; token semantic identity; Saga/lore progression; type/characteristic changes; temporary durations; linked exile-return; permanent-to-graveyard turn history; trigger instances/nested choices; graveyard activation; source-derived replacement. Exact reusable fields/owners UNKNOWN / REQUIRES_FOCUSED_FOLLOWUP. No card-specific fields approved.

## 7. Full-game state-growth census

| Fact | Classification | Owner/horizon |
|---|---|---|
| Ordered library/deck | EXISTING_AUTHORITATIVE_STATE for runtime zone order; immutable deck list is content data | Zones + bundle identity; H5 initializer proof |
| Shuffle seed/algorithm/cursor | REPLAY_CONTROL_DATA plus existing RandomStateV1 cursor | H5; preserve seed/cursor distinction |
| Starting player | REPLAY_CONTROL_DATA / initial setup identity; active player exists at runtime | Do not duplicate unless future rule needs history |
| Opening hands | EXISTING_AUTHORITATIVE_STATE (zones/knowledge) | H5 |
| London mulligan round/keep/bottom selection | DECISION/CONTINUATION_STATE | Typed cross-choice setup continuation needed; exact owner pending focused design |
| First-player initial draw exception | DERIVED_STATE_ONLY candidate | Prove from turn state; no extra flag without need |
| Land reset / mana emptying | NEW_AUTHORITATIVE_STATE_REQUIRED | H1 typed state and step/phase transitions |
| Terminal outcome | EXISTING_AUTHORITATIVE_STATE | EpisodeStatus/checkpoint; no duplicate field |
| Deck legality/certification | IMMUTABLE_CONTENT_DATA | Bundle/certification identity, not EngineState absent authority |

## 8. Decision, event, observation and persistence analysis

Decision V2 response can likely remain generic, but CandidateIntent is closed. PlayLand is a special action, not CastSpell or SelectObject; add typed action/request and authoritative binding. PlayerStep V2 embeds the request/action vocabulary, so needs a successor. Validate actor, active player, phase, priority, empty stack, hand membership, entitlement and definition type. Mana-source activation is a player choice when multiple sources exist; expose complete candidates. No AutoPay, first-source, hidden choice or collection-order fallback. Existing ChooseOne/Many/Number/Order are reusable only when exact domain semantics fit. Cast, payment/source, target, mode, X, may/decline, card selection, ordering, attacks/blocks, replacement, trigger ordering and search/reorder need per-family review.

| Decision family | Current sufficiency | Earliest |
|---|---|---|
| Pass priority | Existing | None |
| Play land | Missing closed intent | H1 request/PlayerStep |
| Basic mana ability | Intent exists, binding incomplete | H1 typed derivable binding |
| Cast spell | Incomplete stack/cost/target semantics | H2 |
| Target selection | Generic selection may encode a candidate response, but target domains/legality and opaque candidate completeness are not established | H2/H3 focused review |
| Mana source / payment | No AutoPay; candidate choice and restriction-aware payment are not yet implemented | H2/H3 focused review |
| Modes | Existing choice domains may suffice for a finite mode set; no selected M4.2 need | Defer until card requires it |
| X | Number response exists, but legal domain/cost binding is not established | Defer until card requires it |
| May/decline | Boolean response may fit; trigger timing/continuation still needs proof | Defer until card requires it |
| Card selection | Generic selection is insufficient without zone/identity privacy and completeness proof | Defer until card requires it |
| Ordering | Existing Order response may fit a fully enumerated legal domain | Defer until card requires it |
| Attacks / blocks | Bounded families exist | Verify against locked card reachability |
| Replacement choice | No complete family established | Defer to reachable requirement |
| Trigger ordering | No complete family established | Defer to reachable requirement |
| Search / reorder | No complete family established | Defer to reachable requirement |

H1 event/delta evidence must include land played, zone/incarnation, tap cost, mana production/clearing and resulting revision. StateDelta binds V5 before/after digest and EngineStateParts replacement, so types must advance with digest. Add typed event/operation only where semantic path requires it. Event enum growth alone does not force replay version if not serialized in versioned replay.

Mana is public (CR 106.4b). New Magic payload codec can contain public pool and legal choices. ObservationEnvelopeV1 (codec ID + bytes), ObservationDigest and InformationStateDigestV2 (observation + retained knowledge) can remain if no envelope/knowledge semantics change. Use opaque request-local IDs only. Never expose GameObjectId, PhysicalCardId, AbilityInstanceId, raw GameState, internal CardDefinition IDs, capability closure, bindings or continuation internals. Nonacting perspectives see only authorized public data; require hidden-world noninterference.

Every new authoritative fact must enter EngineState validation and full digest; checkpoint/digest bind it; replay initial/step/final identities and schema inventory bind successor contracts. Checkpoint→restore→fork with same response must reproduce state/event/delta and next Decision. Do not rely on allocation order, hash iteration, ephemeral request IDs, debug format or pointer identity.

| Old family | Recommended disposition | Migration |
|---|---|---|
| FullStateDigestV5 | READABLE_VERIFIABLE_ONLY, exact detached V5 validation | None authorized |
| EnvironmentCheckpointV6 / CheckpointDigestV6 | New-runtime execution unsupported; archived matching runtime; detached exact verification where codec supports | None authorized |
| Replay V6 | READABLE_VERIFIABLE_ONLY detached; archived-runtime execution | None authorized |
| Decision/request/PlayerStep V2 | Exact historical schema; no reinterpretation | None authorized |
| Observation envelope/digest V1 and InformationStateDigestV2 | Remain executable for old payloads | New named codec, no migration |
| ContentContractIdV1, ExecutionIdentityV1, SemanticContract V1, RNG V1 | Keep schema/meaning if reserved profile seam followed | No cut implied |

## 9. Contract-change matrix

| Contract | Current version | Change | Earliest | Kind/reason and new content | Can remain / cut? | Dependencies | Historical disposition | Confidence |
|---|---|---|---|---|---|---|---|---|
| EngineState | Current aggregate/FSD5 meaning | REQUIRED | H1 | Add pool + land-use | No / cut | Digest/checkpoint/replay | Old meaning immutable | High |
| FullStateDigest | V5 | REQUIRED | H1 | Closed canonical input adds state | No / cut | EngineState | READABLE_VERIFIABLE_ONLY | High |
| StateDelta | V5 digest refs | REQUIRED | H1 | Typed before/after and full replacement | No / coupled type cut | Digest successor | Historical product | High |
| EnvironmentCheckpoint | V6 | REQUIRED | H1 | New state snapshot | No / cut | Digest successor | Old runtime only | High |
| CheckpointDigest | V6 | REQUIRED | H1 | Input binds successor state digest | No / cut | FSD/checkpoint | Exact detached | High |
| Replay manifest/step/container/recorder | V6 | REQUIRED | H1 | New state/checkpoint identity | No / cut | FSD/checkpoint/wires | Detached/archived runtime | High |
| InitialEnvironmentIdentity | V6 | REQUIRED | H1 | New state/checkpoint types | No / cut | Replay successor | Exact historical | High |
| Decision request/CandidateIntent | V2 | REQUIRED | H1 | PlayLand intent/binding | No / cut | PlayerStep | Exact historical | High |
| DecisionResponse | V2 | NO_CHANGE | H1 | Generic response can remain | Yes / no | New request references | Exact | Medium-high |
| Player decision wire | V2 | REQUIRED | H1 | Land action request | No / cut | Decision | Exact historical | High |
| PlayerStep | V2 | REQUIRED | H1 | Embeds request/action/event | No / cut | Wires/events | Exact historical | High |
| Observation envelope | V1 | NO_CHANGE | H1 | Codec field already exists | Yes / no | New payload codec | Exact | High |
| Observation payload | Named codecs | REQUIRED | H1 | New public pool/candidate codec | Old remain / new codec | Observation projection | Immutable IDs | High |
| ObservationDigest | Current | NO_CHANGE | H1 | Hashes canonical bytes | Yes / no | Envelope | Exact | High |
| InformationStateDigest | V2 | NO_CHANGE | H1 | New view in existing observation | Yes / no | Envelope | Exact | High |
| Authoritative events | Closed Rust enum | REQUIRED | H1 | Typed land/mana events | Enum grows; no independent wire cut proved | Rules products | Preserve old variants | High |
| SemanticDeltaOperation | Closed Rust enum | REQUIRED | H1 | Typed land/mana operations | Enum grows; no independent wire cut proved | StateDelta | Preserve old variants | High |
| ExecutionIdentity | V1 | NO_CHANGE | H1 | Values only | Yes / no | Semantic contract | Exact | High |
| SemanticContract | V1 | NO_CHANGE | H1 | Existing fields bind new IDs | Yes / no | Content/rules values | Exact | High |
| RulesContract | Current manifest | NO_CHANGE | H1 | Capability keys/versions values | Yes if fields fit | Registry | Exact | Medium-high |
| ContentContract/CardDefinition | V1 | NO_CHANGE | H1 | Reserved typed profiled seam | Yes if exact seam honored | Profile | Unprofiled bytes exact | High |
| CardSemanticProfile | None executable | REQUIRED | H1 | First bounded typed profile | New contract | Content/requirements | New | High |
| Capability Registry | V1 | NO_CHANGE | H1 | Add typed entries, no schema change | Yes | Owners | Old entries exact | Medium-high |
| RNG | V1 | NO_CHANGE | H1 | No randomness | Yes | None | Exact | High |
| FormatState | Optional slot | NO_CHANGE | H5 | Certification not runtime state | Yes | Bundle | Exact | Medium-high |

## 10. Minimal cut recommendation

### CUT_A — before M4.2 implementation

One coordinated state identity cut should cover only proven H1 plus immediate shared H2 mana facts:

* Per-player six-color ManaPool with finite restriction class unrestricted / creature-spell-only (Rockface Village is in locked R1).
* Per-player land-play-used-this-turn fact with bounded default entitlement/reset semantics.
* No persistent ability registry: derive static ability from object incarnation + content-scoped CardDefinition + FaceKey + AbilityKey; repair typed candidate binding.
* No new continuation, effect record, allocator or pregame state.

Move together: EngineState, FullStateDigest input/type, StateDelta digest/replacement types, EnvironmentCheckpoint, CheckpointDigest, Replay manifest/step/container/recorder/schema inventory, InitialEnvironmentIdentity. This report assigns no version numbers. Old versions retain exact meaning; no automatic migration. FSD5 is detached verifiable; old checkpoint execution requires archived matching runtime; Replay V6 detached/readable-verifiable and archived-runtime executable.

Separately, PlayLand request/action wire and PlayerStep require named schema/type successors. Generic DecisionResponseV2 can remain. ObservationEnvelope, ObservationDigest and InformationStateDigestV2 can remain; add a named Magic payload codec. Authoritative event/delta enums grow for land/mana evidence without independently forcing replay cut.

### CUT_B — consolidated before stateful R1/W1 implementation

Oracle proves further state needs, but exact reusable schema/invariants are not frozen. Before the first affected M4.3/M4.4 card implementation, conduct one coordinated design for counters, turn histories, face/transformation, durations, attachments, generated-object identity, trigger/stack payload and linked exile only to reachable needs. If authoritative state is added, expect another FullStateDigest/StateDelta/checkpoint/checkpoint-digest/replay cut. Do not add speculative generic bags in CUT_A; a later justified cut is preferable to ambiguous frozen state.

### H5 setup boundary

London mulligan round/keep/bottom choices require a typed continuation across choices and likely a state identity cut. Its owner is not mature enough for CUT_A/B. Starting player, first-draw exception and outcome are not added absent proof. A setup-state audit may establish another cut.

| First-cut question | Finding |
|---|---|
| ManaState? | Yes, six colors plus known closed restriction class. |
| Land-play usage? | Yes; cannot reconstruct after land leaves. |
| Persistent ability-instance authority? | No; derivable identity sufficient, current binding is a gap. |
| Turn-action state? | Land use only now; other histories in CUT_B. |
| New allocator? | No. |
| Continuation/effect record? | No for atomic land/mana. |
| Pregame state? | No. |
| Digest/StateDelta/checkpoint/checkpoint digest/replay? | Yes, coupled to new EngineState. |
| Decision successor? | New PlayLand request; generic response may stay. |
| PlayerStep successor? | Yes, current contract embeds request/action vocabulary. |
| Observation successor? | New payload codec only; envelope/digests may stay. |

## 11. Deferred semantics, non-goals, stop conditions

Deferred/unknown: typed schemas for counters, attachments/links, generated tokens, face identity, durations, trigger/stack payload, replacement layers, later target/payment domains, and mulligan continuation. Prove Optimistic Scavenger Room reachability before adding Room semantics. Review exact CR clauses when each card family is selected. Create repository-owned deck provenance manifest before content bundle freeze.

This audit does not authorize all R1/W1 cards, universal mana/payment/casting/stack/target/effect languages, all keywords/replacements/continuous effects, full setup, zero-unsupported certification, bundle freeze, M5, optimization, or production implementation.

Stop for architecture review if M4.1’s reserved profile seam is insufficient, closed typed contracts cannot represent chosen state, official authority contradicts engine semantics, ability candidates cannot be bound safely, RulesKernel cannot own the transition, or reusable state has no precise owner/invariants.

## 12. Recommended next task

Create a focused **M4.2 state-and-version-cut design** resolving exact ManaPool and land-use fields, validation/canonical ordering, phase clearing, RulesKernel requests/products/events/deltas, PlayLand request and PlayerStep schemas, payload codec, and checkpoint/replay successor compatibility. Reconcile #225’s empty body/status discrepancy. This design should be accepted before any version implementation or real-card execution. Accept the consolidated CUT_B state design before implementing stateful R1/W1 cards.
