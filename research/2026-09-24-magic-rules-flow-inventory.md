# MAGIC_RULES_FLOW_INVENTORY_V1

- **Status:** RESEARCH / PLANNING / NON-NORMATIVE
- **Baseline:** branch `chris/m3-s3-a-ordered-sba`, exact `HEAD = 8cafb91b121cc2e29f3837fffbd8a189cbf817`
- **Reference master:** `66f3b713787cad89674257f6e0b6448b9fd568f9`
- **Rules authority:** `wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f`

```text
DOES_NOT_CHANGE_CAPABILITY_LIFECYCLE = YES
DOES_NOT_AUTHORIZE_IMPLEMENTATION = YES
PRODUCTION_RUST_CHANGED = NO
PRODUCTION_PYTHON_CHANGED = NO
```

The pinned authority is the official [Wizards Comprehensive Rules TXT,
effective August 7, 2026](https://media.wizards.com/2026/downloads/MagicCompRules%2020260819.txt).
Its exact URL, measured digest and provenance are recorded in [ADR 0051](../docs/adr/0051-comprehensive-rules-snapshot-authority.md)
and the [snapshot identity record](../docs/rules/COMPREHENSIVE_RULES_ARTIFACT_RESEARCH_2026-09-15.md).
The matching official PDF is [MagicCompRules 20260807.pdf](https://media.wizards.com/2026/downloads/MagicCompRules%2020260807.pdf).
This inventory cites rules but does not reproduce their text or promote
research into semantic authority.

## 1. Method and status meanings

This inventory follows the chronological game flow, then records cross-cutting
systems. Implementation location is checked against source at the exact branch
head, not inferred from plans or registry prose. It separates the S3.A branch
candidate from code admitted under the production S1 identity.

For horizon columns, `R = REQUIRED`, `D = DEFER`, and `U =
UNKNOWN_PENDING_CARD_REVIEW`. Current status has exactly these meanings:

| Status | Meaning |
|---|---|
| `HAVE` | The bounded semantic surface in the row is sufficiently implemented for that declared scope. |
| `PARTIAL` | Some authoritative state, protocol, validator, or producer exists, but the flow is incomplete. |
| `MISSING` | A required semantic surface has no authoritative implementation. |
| `DEFER` | The surface is intentionally outside the selected target. |

Implementation location is one of `MERGED_MASTER`, `BRANCH_REVIEWED`,
`BRANCH_CANDIDATE`, `DESIGN_ONLY`, `RESEARCH_ONLY`, or `ABSENT`. The status and implementation-location category are separate columns.
`EXISTING_CAPABILITY / CODE_LOCATION` carries the capability name and, where
known, the authoritative source module. Each row retains all requested fields;
`I` and `FC` label `KNOWN_INTERACTIONS` and `FAIL_CLOSED_BOUNDARY`, while
`Owner`, `Milestone`, and `Note` remain distinct within the last cell.

### Horizons

| Horizon | Inventory target |
|---|---|
| **A — M3 exit** | The accepted format-neutral, two-player, synthetic inert-card normal-turn path beginning from the validated turn-2 state. All 11 Foundation capabilities must reach their declared covered exit; this horizon does not add a pregame. |
| **B — first playable basic game** | Two players, real initialized decks, opening hands/mulligan, normal turns, land play, a deliberately bounded basic land/vanilla-creature card pool, enough mana/casting/stack/resolution to play it, combat, SBA, and terminal results. The initial card pool may exclude triggers, replacements, complex effects and targeted spells. |
| **C — researched R1 × R1** | A future exact Mono-Red R1 mirror candidate. Card-specific closure remains `UNKNOWN_PENDING_CARD_REVIEW`; the advisory 69-capability research closure is not an implementation queue. |

R1 is the Endo Takatomo Mono-Red burn/Ojer aggro research candidate (60-card
list, 12 unique names). The source list is named inside an archived research
package but is not present as a tracked deck file at this HEAD. The research
snapshot explicitly says that the pair is unfrozen and that no cards, registry
entries, bundle, or certification resulted. See [the reviewed research
snapshot](2026-09-15-m3-inputs/standard-matchup-research.md) and its
[non-normative readme](2026-09-15-m3-inputs/README.md).

## 2. Executive summary

### A. What Manafold already has

| Layer | Inventory result |
|---|---|
| **Foundational engine substrate** | Complete `EngineState`, canonical IDs and allocators, HMAC counter RNG, ordered-zone state, opaque identity/knowledge lifecycle, generic Decision V2 and typed continuations, exact StateDelta, semantic event cursor, V6 checkpoint/replay, player endpoints, and one atomic response transaction with at most one forced-progress call. These are infrastructure, not proof of full Magic semantics. |
| **Magic state substrate** | `TurnPosition`, life/`has_lost`, priority holder/pass count, bounded CombatState, simple creature source facts and marked damage, object/physical-card identity, zones/ordered libraries, stack record slots, effects/triggers/delayed-effect slots, and `FormatState`. The structure deliberately lacks land/card types, mana, costs, rules text, targets as legal objects, spell payloads, characteristic layers, damage assignment history, and general trigger/replacement state. |
| **Magic semantics** | Production `rules/turn-structure@0.1.0` is covered only for the narrow S1 turn/untap/cleanup profile. S2's two selected zone-incarnation families are implemented but not covered. S3.A has reviewed continuation validation, APNAP Order staging and Magic APNAP observation on this branch; atomic SBA application, production S3 admission, fixed point and production replay/restore parity are not implemented. |

### B. What M3 still needs

1. Complete the S3.A atomic simultaneous SBA batch, including no-order and
   final-Order paths, `has_lost`, terminal mapping, ordered S2 composition,
   bounded post-damage combat-reference pruning and rollback evidence.
2. Complete S3.A fixed-point, checkpoint/restore/fork/replay evidence and the
   production S3 semantic contract. Task 9B0 explicitly blocks nonterminal
   final-Order environment/replay claims until S3.B or a reviewed transaction
   disposition because forced progress reaches the missing Basic Priority
   boundary.
3. Implement the specified two-player pass-only Basic Priority kernel and its
   event/delta/cursor contract.
4. Admit the new Magic contract through V6 and wire Reference player responses
   through the existing response transaction; prove actor/nonactor products,
   rejection atomicity and replay.
5. Implement ordinary Draw through the one S2 workspace; integrate
   Upkeep-pass → Draw → active-player priority without an environment loop.
6. Supply the genuine V6 S2 transition replay witness and complete SBA/Draw ×
   priority/zone/information interactions; only then can S2 coverage and the
   remaining capability evidence be reviewed.
7. Run cumulative M3 gates and exact-head review; promote no lifecycle until
   its evidence passes.

### C. What the first playable game still needs

Beyond M3, the minimum coherent work is: real deck/game initialization,
starting-player selection, shuffle, opening hand and mulligan; a reviewed
minimum card/content representation; land play and mana; spell legality/cost
payment/casting/stack/resolution; then an end-to-end card-pool game with
terminal, checkpoint, replay, information and certification evidence. The
first content closure must be intentionally small and may exclude mechanics
that none of its cards require. “Playable” must name that exact card and rule
closure rather than imply arbitrary Magic.

### D. What R1 adds beyond the playable shell

The research indicates a full R1 mirror adds burn/target interactions,
damage-source and recipient classification/history, the Ojer package's
damage/lifecycle interactions, exact Oracle definitions, Standard legality,
and a locked deck/bundle/benchmark identity. These are planning signals only.
The exact twelve card names, text, and per-card requirements were not imported
into this repository, so their closure is `UNKNOWN_PENDING_CARD_REVIEW`. The
research's 69 transitive candidate capabilities are not a queue or support
claim.

### E. What to explicitly defer

- M3: pregame, actual decks/cards, land play, mana, casting, stack, general
  targets/effects, arbitrary triggers/replacements/layers/copy, and card or
  format support. M3's validated synthetic turn-2 inert placeholder remains
  the target.
- First playable: multiplayer, Commander, sideboards, arbitrary keywords,
  first/double strike, multiple blockers, advanced damage assignment,
  continuous/copy layers, general replacement/prevention and cards requiring
  those surfaces, unless the selected minimum card pool forces a subset.
- R1: asymmetric R1×W1 is a later benchmark candidate; R1×R1 itself is
  provisional pending exact card/deck and interaction review.
- Later milestones: search/determinization, training/reward/trajectory
  production, optimized rollout backends, loops/shortcuts, certification of
  arbitrary bundles, multiplayer variants, Commander and all-card support.

## 3. Chronological rules-flow inventory

For `CURRENT_STATUS`, `HAVE` in a row means only the row's named bounded
surface. State shape, a generic DTO, a testkit, or a typed event family is not
semantic support for the entire row. `SBA` means
`rules/state-based-actions-combat@0.1.0`; `Z` means
`rules/zone-incarnation@0.1.0`; `TP` is the current M3 S3 interaction plan.

### Pregame

| FLOW_ID | FLOW_POSITION | CR_AUTHORITY | RULE_MEANING | A / B / C | CURRENT_STATUS | IMPLEMENTATION_LOCATION | EXISTING_CAPABILITY / CODE_LOCATION | EXISTING_STATE_SUBSTRATE | EXISTING_DECISION_SUBSTRATE | EXISTING_EVENT_DELTA_SUBSTRATE | EXISTING_OBSERVATION_SUBSTRATE | WHAT_IS_MISSING | KNOWN_INTERACTIONS | FAIL_CLOSED_BOUNDARY | RECOMMENDED_OWNER | RECOMMENDED_MILESTONE | NOTES |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| PG-01 | Create game and bind two players/decks | CR 100.1–100.2a, 103 | Construct a game from declared players and complete decks. | D / R / R | MISSING | ABSENT | — | SyntheticResetInputs only takes exactly two PlayerIds and seed/setup. | No game-start request. | No game-start event. | PlayerStep exists after state, no setup view. | Deck ingestion, complete card objects, game identity, real initial zones. | Do not treat the synthetic two-object constructor as a Magic game. | Deck content × shuffle × library/knowledge × replay identity. | Game setup / content | M4-Foundation | `construct_synthetic_engine_state` starts a fixed synthetic state. |
| PG-02 | Choose starting player | CR 103.1, 103.8 | Starting player/turn order is determined before first turn. | D / R / R | MISSING | ABSENT | — | `active_player` is stored; constructor fixes P1. | No coin/die/agreement/start-player Decision. | No start-player audit. | No pregame observation. | Deterministic, replayed starting-player procedure/policy. | Reject unproven first-turn context; S1 begins from validated state. | Starting-player choice × turn-1 skip × deterministic episode identity. | Game setup / turn | M4-Foundation | Not the S1 active-player field. |
| PG-03 | Shuffle decks | CR 103.3 | Each starting deck is randomized into a library. | D / R / R | MISSING | ABSENT | — | Ordered library vectors and RNG state exist. | No shuffle operation/Decision. | No shuffle event or random-state-to-deck transition. | No library-order reveal. | Define deterministic shuffle stream, algorithm, replay identity and hidden-information behavior. | No sorting or retaining source deck order as a Magic shuffle. | RNG stream × library identity/order × hidden-information noninterference × replay. | Randomness / zones | M4-Foundation | RNG service is substrate only. |
| PG-04 | Starting life and initial hand | CR 103.4–103.5 | Initialize format life total and draw starting hand. | D / R / R | MISSING | ABSENT | — | Life and Hand/Library zones exist; reset uses life 40 in synthetic fixture. | No starting draw/player initialization path. | No initial draw occurrence or `ZoneTransition` producer. | Knowledge DTO exists, no game-start private hand projection. | Standard default life 20, card locations, owner knowledge, exact initial products. | Do not infer Magic starting life/hand from synthetic reset. | Initial draw × Library→Hand S2 × owner-private identity/VisibleSequence. | Game setup / draw | M4-Foundation | Commander 40 is irrelevant to Standard R1 and out of scope. |
| PG-05 | Mulligan | CR 103.5 | APNAP mulligan declarations; redraw; put a count on library bottom in chosen order; repeat. | D / R / R | MISSING | ABSENT | — | Ordered Library and Hand locations exist. | `Order`/`ChooseOne` can represent pieces, but no mulligan protocol/continuation. | No mulligan event/delta/replay flow. | Opaque/knowledge substrate exists; no opening-hand exchange projection. | Simultaneous declarations, bottom-order choice, repeated rounds, identity/knowledge and replay. | Do not default keep, choose bottom cards, or partially mutate a round. | APNAP declarations × simultaneous redraw × bottom-order choice × private hands. | Game setup / decision / zones | M4-Foundation | Higher-priority APNAP/mulligan hidden-zone contract. |
| PG-06 | Pregame actions and first-turn draw skip | CR 103.2, 103.6–103.8a | Process supported pregame actions and skip first player's first draw. | D / R / R | MISSING | ABSENT | — | `TurnPosition` and turn number exist. | No starting effect action/Decision. | No game-start / skip event cursor. | No pregame surface. | Start procedure, first-turn marker and skipped-draw state derivation. | M3 explicitly starts from a validated turn-2 state; do not infer a skipped draw. | Starting player × draw skip × first TurnPosition/turn number × replay. | Game setup / turn | M4-Foundation | No current “first turn” fact in EngineState. |

### Turn frame, beginning phase and main phases

| FLOW_ID | FLOW_POSITION | CR_AUTHORITY | RULE_MEANING | A / B / C | CURRENT_STATUS | IMPLEMENTATION_LOCATION | EXISTING_CAPABILITY / CODE_LOCATION | EXISTING_STATE_SUBSTRATE | EXISTING_DECISION_SUBSTRATE | EXISTING_EVENT_DELTA_SUBSTRATE | EXISTING_OBSERVATION_SUBSTRATE | WHAT_IS_MISSING | KNOWN_INTERACTIONS | FAIL_CLOSED_BOUNDARY | RECOMMENDED_OWNER | RECOMMENDED_MILESTONE | NOTES |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| TF-01 | Turn/phase/step frame | CR 500–501, 505, 512 | Phases and steps occur in order; active player/turn change at turn boundary. | R / R / R | PARTIAL | MERGED_MASTER | `turn-structure`; turn-structure | Closed `TurnPosition`, active PlayerId, turn number. | None for temporal advance. | Typed position/turn/active-player events and cursor in S1. | Synthetic M3 position/priority codec. | Complete phase progression remains separated by missing Priority/Draw/Combat producers. | Priority-bearing vs no-priority steps; turn-based action before priority. | S1 admission rejects all unsupported downstream states. | turn orchestrator | M3 then M4 | S1 covers the bounded temporal/untap slice only. |
| TF-02 | Untap | CR 502.2–502.4 | Ordinary eligible active-player permanents untap simultaneously without priority. | R / R / R | HAVE | MERGED_MASTER | `turn_structure.rs`; turn-structure | tapped, controller, active player, bounded control history. | No untap Decision in bounded case. | `UntapCompleted`, occurrence/tap audit, exact delta. | Temporal S1 view; tap occurrences use existing projections. | Add full combat/game context only through later reviewed interactions. | No-untap modifiers, phasing/day-night and effects fail closed/out of scope. | Control history × ordinary untap × tap occurrences × no-priority boundary. | turn | M3 | S1 conformance is covered for its declared boundary. |
| TF-03 | Upkeep actions, SBA gate and priority | CR 503.1, 117.5, 704 | Upkeep turn-based/trigger work; SBA checks before priority; then priority if supported. | R / R / R | PARTIAL | BRANCH_CANDIDATE | S1 `turn_structure`; S3.A candidate `magic`/`state_based_actions`; turn-structure, basic-priority, SBA | phase/position, life, creature facts, continuation. | Task 7 can stage nonfinal SBA Order in candidate. | Order-created/cleared/chosen; no SBA-batch event/producer yet. | Branch-reviewed APNAP current-state codec; no production S3 route. | SBA producer/batch/terminal/priority sequence and environment composition. | Task9B0 no-order combat-policy RED remains open; unsupported priority must abort. | SBA → S2 → pass-only gate → environment forced progress/replay. | state_based_actions + priority | M3 | Task 9B0 not implemented. |
| TF-04 | Draw step | CR 504.1, 121.1–121.4 | Active player draws one, changing Library/Hand identity and knowledge; empty-library draw loses at next priority. | R / R / R | PARTIAL | MERGED_MASTER | S1 temporal step + S2 executor only; turn-structure, draw-card, Z | ordered Library, Hand zone, physical/GameObject IDs. | No Draw/empty-library Decision. | S2 `ZoneTransition` can move library top to hand; no Draw producer/event disposition locked by Task 15. | Owner-private identity acquisition/remap machinery; no Draw-step projection producer. | Draw timing, empty-library loss, priority-after-draw, no-repeat and replay integration. | S1 stops at Draw; S2 rejects wrong top/source. | Fail before mutating on empty/unsupported library. | turn / zones_identity | M3 | S2 standalone move is not a Draw rules capability. |
| TF-05 | Precombat/Postcombat Main and priority | CR 505.1–505.2, 117 | Main phase gets priority; sorcery timing depends on active turn, priority and empty stack. | R / R / R | PARTIAL | MERGED_MASTER | phase enum S1; `basic-priority` design; turn-structure, basic-priority | phase/active player/priority. | PassPriority intent exists; no production Priority kernel. | No typed PriorityChanged event/cursor. | Turn position only, no action legality surface. | Pass progression and action windows, plus main-phase actions. | S1 downstream main state rejects at BasicPriority. | Priority window × Draw/land/casting; land play is a special action, not a stack spell. | priority owner | M3 / S3.B | Land/cast are separate turn actions, not phase changes. |
| TF-06 | Land play | CR 305.1–305.4, 116.2 | Priority holder plays a land in own main phase with empty stack; normally one per turn; special action, not a spell. | D / R / R | MISSING | ABSENT | — | Zone supports Battlefield/Hand; no card type or per-turn land count. | Generic SelectObject possible; no legal land set. | No land-play state/event/cursor. | No hand/land candidates in Magic observation. | Land types, land-play entitlement/count, active player/timing and placement. | Do not move an arbitrary Hand object to Battlefield. | Priority + Main + empty stack × per-turn allowance × land type and mana abilities. | turn action / zones | M4-Foundation | Basis for real basic decks. |
| TF-07 | Mana and mana abilities | CR 106, 605, 117.3d | Mana pool, mana type/amount, abilities, activation and emptying are authoritative. | D / R / R | MISSING | ABSENT | — | No ManaPool, colors, symbols, ability program or land types. | ActivateAbility intent exists without legal candidates. | No mana/ability event or semantic delta. | No mana/card ability observation. | Mana state, tap/activation, production, restrictions, payment and per-step emptying. | Do not fake cost payment by reducing a numeric total. | Lands/mana abilities × activation costs × priority windows × spell payment. | mana / rules | M4-Foundation | Generic RNG/state does not provide mana semantics. |
| TF-08 | Spell casting and timing | CR 601, 302.1, 304.1 | Propose legal spell, put on Stack, choose required values/targets and proceed through casting steps. | D / R / U | MISSING | ABSENT | — | Stack records lack spell object payload/type/cost. | CastSpell intent exists, but no rules-derived cast candidates. | No SpellCast/stack cursor operation. | No stack/hand card action view. | Casting legality, card types, timing, announcements and rollback. | Land play is not spell casting; only card-pool-supported casting should be admitted. | Reject before mutation if any casting step lacks authority. | spell / stack | M4-Foundation | R1 card-specific scope pending review. |
| TF-09 | Costs and payment | CR 118, 601.2f–h, 602 | Determine total/alternate/additional costs; pay costs in valid order atomically. | D / R / U | MISSING | ABSENT | — | No mana pool/cost-expression state. | ChooseNumber can encode a number but not payment/choice semantics. | No payment audit/cursor. | No cost/payment explanation. | Mana accounting, cost modifiers, sacrifice/discard/tap costs and rollback. | Do not count a card as cast on partial payment. | Fail before mutation on unsupported costs. | costs / mana | M4-Foundation | Card IR cost vocabulary is not frozen. |
| TF-10 | Targets, modes and choices during casts/resolution | CR 115, 601.2b–d, 608.2b, 700.2 | Legal target sets/modes/choices bind to current state and are rechecked on resolution. | D / R / U | PARTIAL | MERGED_MASTER | Decision V2 substrate only; Decision V2; future target capability | Opaque/trusted object/player identity maps. | SelectObject, SelectPlayer, SelectMode and ChooseOne/Order types exist. | No target legality/target event cursor. | Generic opaque candidates exist; Magic legality/explanations absent. | Target filters, restrictions, mode/card-specific choice, target legality recheck and illegal-target resolution. | Candidate DTO support does not imply legality. | Never offer incomplete/heuristic targets. | targets / decision | M4-Foundation / M4-R1 | B initial pool may avoid target spells; this inventory's B assumes target support for a playable spell pool. |
| TF-11 | Stack and priority passes | CR 405, 117.1–117.5 | Stack is LIFO; passes in succession while empty close a priority window; responses add stack objects. | D / R / R | PARTIAL | MERGED_MASTER | State storage + Decision V2; kernel behavior specified; basic-priority | `StackRecord`, `stack_order`, PriorityState. | PassPriority and CastSpell/ActivateAbility intent shapes. | No PriorityChanged/SpellCast/resolve event or cursor. | S1 observes only priority field; no stack/player presentation. | Stack payload, order/resolve/counter transitions, priority holder pass counter and action generation. | Current S1/S3 environment boundary fails closed at BasicPriority. | No automatic pass or stack resolution. | priority / stack | M3 then M4 | Storage-only stack is not a functional stack. |
| TF-12 | Spell and ability resolution | CR 608–610 | Resolve top object, check targets, apply effects, move spells/abilities to destination, trigger follow-up. | D / R / U | MISSING | ABSENT | — | Stack slot exists; no effect interpreter or spell payload. | Choices generic only. | No resolution/event/delta semantics. | No resolving object/target lifecycle projection. | Resolution state machine, effect subset, illegal target behavior, zone/LKI/trigger coupling. | Fail closed when an effect cannot be interpreted. | Target legality × LKI × triggers × subsequent SBA × spell destination. | spell resolution | M4-Foundation | Basic no-target permanent spells still need the minimal permanent-resolution path. |
| TF-13 | Activated abilities and special actions | CR 116, 602, 605 | Validate/announce/pay/resolve activated ability; mana abilities have special timing; special actions bypass stack as specified. | D / R / U | PARTIAL | MERGED_MASTER | Identity/Decision substrate; no activated-ability capability | Ability IDs/mappings and candidate intent enum only. | ActivateAbility exists without ability programs/legality. | No activation/mana event cursor. | No ability surface projection. | Ability definitions, activation costs, priority exceptions, per-turn limits and resolution. | Distinguish mana abilities from ordinary activated abilities and special actions. | No arbitrary ability IDs become executable. | abilities / mana | M4-Foundation | R1 interaction unknown pending card review. |
| TF-14 | Triggered and delayed abilities | CR 603–604, 603.3, 603.7, 101.4 | Detect triggers from events/state; order APNAP; stack them; handle intervening conditions and delayed triggers. | D / U / U | PARTIAL | MERGED_MASTER | Typed placeholder state only; none implemented | WaitingTrigger has ID/controller; delayed/effect records have ID/label only. | Decision Order can represent ordering but no trigger producer. | No trigger event/source/ability/cursor. | No trigger/stack observation. | Trigger conditions/source snapshots/APNAP stack placement/order, delayed scheduling and replay. | Do not infer ability semantics from labels/IDs. | Reject any populated unsupported trigger path in current profile. | triggers | M4-Foundation / M4-R1 | Capabilities currently exclude triggers; R1 exact card review needed. |
| TF-15 | Replacement and prevention | CR 614–616 | Replace/prevent proposed events before they commit, with ordering/choice where needed. | D / U / U | MISSING | ABSENT | none | No replacement/prevention state. | No replacement ordering Decision producer. | No replaced/proposed event pipeline. | No surface. | Typed replacement effects, applicable set, APNAP/order, deterministic event transformation. | Never apply base damage/zone consequence when replacement is unproven. | Reject unsupported effects before mutation. | replacement / prevention | M4-Foundation / M4-R1 | Full spells/card text can introduce these. |
| TF-16 | Continuous effects and copy values | CR 611–613, 707 | Derive effective characteristics in layer/dependency order; preserve copy characteristics. | D / U / U | MISSING | ABSENT | none | Only FoundationCreatureSource simple base characteristics and marks. | No layer-choice surface. | No continuous/copy event/cursor model. | No card characteristic observation beyond authorized known definitions. | Layer system, timestamps/dependencies, copiable values, dynamic recalculation and SBA interaction. | Current SBA reads base values only after proving excluded axes absent. | Fail closed for any modifier/copy semantics. | characteristics / card IR | M4-Foundation / M4-R1 | Major dependency for a wider card pool. |

### Combat

| FLOW_ID | FLOW_POSITION | CR_AUTHORITY | RULE_MEANING | A / B / C | CURRENT_STATUS | IMPLEMENTATION_LOCATION | EXISTING_CAPABILITY / CODE_LOCATION | EXISTING_STATE_SUBSTRATE | EXISTING_DECISION_SUBSTRATE | EVENT_DELTA_SUBSTRATE | OBSERVATION_SUBSTRATE | WHAT_IS_MISSING | KNOWN_INTERACTIONS | FAIL_CLOSED_BOUNDARY | RECOMMENDED_OWNER | RECOMMENDED_MILESTONE | NOTES |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| C-01 | Combat-phase order and skip rules | CR 506, 508.8, 511 | Beginning, declarations, damage, end combat; skip blockers/damage with no attackers. | R / R / R | PARTIAL | MERGED_MASTER | TurnPosition enum; S3 combat design; combat-phase specified | Combat step enum, optional CombatState. | No combat step producer. | No combat progression event product. | Turn position only. | Begin/skip/advance ordering and per-boundary priority/SBA gates. | Attack skip affects whether blockers/damage occur. | S1 forced progress rejects Combat. | combat / turn | M3 | State supports position, not phase semantics. |
| C-02 | Beginning-of-combat action/priority | CR 507.1–507.2, 117 | Establish defender if needed; priority after turn-based actions. | R / R / R | PARTIAL | DESIGN_ONLY | No producer; combat-phase, basic-priority | CombatState.defending_player. | No begin-combat/player action Decision. | No priority event/cursor. | No combat-context view. | Defender/priority flow (two-player defender may be fixed). | No multiplayer target selection. | Unsupported combat boundary. | combat / priority | M3 | Foundation two-player only. |
| C-03 | Declare attackers | CR 508.1, 508.2, 508.8 | Choose a legal subset, tap, bind defending player, handle no attackers. | R / R / R | PARTIAL | DESIGN_ONLY | State substrate + spec; declare-attackers specified | CombatState.attackers, tapped and control-history facts. | ChooseMany supports explicit subsets, not a Magic producer. | No AttackersDeclared semantic event/delta/cursor. | Opaque IDs/Decision V2 generic. | Eligibility derivation, legal candidate completeness, atomic tap/attack apply, skip proof. | Haste, restrictions, costs and >8 attackers excluded in Foundation. | Fail before Decision if eligible set unsupported/unproven. | combat declaration | M3 | Capability lifecycle still specified. |
| C-04 | Declare blockers | CR 509.1a–h, 509.2 | Assign zero/one blocker under reviewed subset; derive blocked state and legal assignment. | R / R / R | PARTIAL | DESIGN_ONLY | State substrate + spec; declare-blockers specified | CombatState blocker map and tapped/control facts. | ChooseOne/Confirm shape can represent one eligible blocker. | No blocker-declaration event/cursor. | Only opaque identities generic. | Eligibility, evasion/restrictions/requirements and assignment producer. | More than one relevant blocker fails closed in Foundation. | No default block/no block. | combat declaration | M3 | Blocked status has historical semantics after blockers leave (509.1h). |
| C-05 | Combat damage assignment | CR 510.1a–c, 510.2 | Freeze legal source/recipient assignment, assign, then deal damage simultaneously. | R / R / R | MISSING | ABSENT | combat-damage specified | Combat participation, simple power/toughness, marked damage. | No assignment/recipient generation. | No assignment/deal event product. | No combat assignment projection. | Complete damage assignment and source/recipient proof. | First/double strike, trample, multiple blocker assignment, prevention excluded. | Reject before damage mutation when any recipient rule is unproven. | combat / damage | M3 | Rules source authority is exact CR snapshot. |
| C-06 | Damage application and life/marks | CR 119, 120.1–120.6 | Apply simultaneous unmodified damage to life and creature marks. | R / R / R | PARTIAL | MERGED_MASTER | State data only; damage-and-life specified | Player life, marked_damage. | No damage choices for bounded basic case. | LifeChanged exists, but no damage source event/complete simultaneous damage cursor. | Life projection general; Magic combat damage view absent. | Damage result producer, source attribution, marks and simultaneous package. | SBA consumes the resulting life/marks; cleanup resets marks. | Damage modifiers/replacement fail closed. | damage | M3 | State values do not mean damage semantics are implemented. |
| C-07 | Post-damage SBA and fixed point | CR 104.3b, 704.1–704.5 | Derive all actions from one snapshot, apply simultaneously, repeat until stable before priority. | R / R / R | PARTIAL | BRANCH_CANDIDATE | Rules validator/APNAP candidate; Task 9B0 test plan; SBA specified | Life/has_lost, FoundationCreatureSource, zones. | Order continuation implemented for staged owners. | SbaGraveyardOrderChosen exists; StateBasedActionsApplied absent. | Magic APNAP current state only. | Atomic batch, `has_lost`, S2 moves, terminal results, no-order path and repeat. | Batch×S2, combat references, priority precondition and replay. | Current producer red; never mutate selected action early. | state_based_actions | M3 | Current branch no production S3 identity/restore. |
| C-08 | APNAP owner choices and Graveyard order | CR 101.4, 101.4b, 404.3 | Collect owner orders active-first; later chooser knows earlier public choices; selected new cards arranged top-to-bottom. | R / R / R | PARTIAL | BRANCH_REVIEWED | Rules `magic.rs`; Magic M3 observation; SBA, Z | Typed Magic SBA continuation, owner order. | Decision V2 Order and actor-only visibility. | SbaGraveyardOrderChosen cursor/event exists; final batch absent. | `magic-m3-observation.v1` uses each perspective's opaque IDs, reviewed branch. | Final owner choice + complete application in one transition. | Owner group moves reverse through S2 workspace. | Incomplete order invalid; trusted IDs never public. | state_based_actions / information | M3 | No production SemanticContractId yet. |
| C-09 | Leaves combat and historical blocked status | CR 506.4, 509.1h, 511.3 | Leaving battlefield prunes combat references; attacker remains blocked if all blockers leave. | R / R / U | PARTIAL | BRANCH_CANDIDATE | Task 9B0 design/tests only; combat-phase, SBA | CombatState has attackers and attacker→Option blocker only. | No choice in accepted bounded cleanup. | StateBasedActionsApplied is designed to authorize exact pruning; not implemented. | No historical blocked-status surface. | Post-damage-only pruning; future pre-damage needs historical blocked representation. | Single damage step/no first-double strike required for bounded simplification. | Earlier combat boundaries and stale EndOfCombat participants fail closed. | combat + SBA | M3 / M4-R1 | `None` after CombatDamage only means no live blocker reference; not a general blocked bit. |
| C-10 | End of combat | CR 511.1–511.3 | End combat priority and remove remaining participants at phase end. | R / R / R | PARTIAL | MERGED_MASTER | TurnPosition only; combat-phase specified | CombatState. | No end-combat priority. | No progression/removal event. | Turn position only. | End-combat transition after supported pass flow. | Earlier SBA removals feed this step; skipped combat has no blockers/damage. | No S1 combat progression. | combat / turn | M3 | Position enum is not a producer. |

### Postcombat and ending

| FLOW_ID | FLOW_POSITION | CR_AUTHORITY | RULE_MEANING | A / B / C | CURRENT_STATUS | IMPLEMENTATION_LOCATION | EXISTING_CAPABILITY / CODE_LOCATION | EXISTING_STATE_SUBSTRATE | EXISTING_DECISION_SUBSTRATE | EVENT_DELTA_SUBSTRATE | OBSERVATION_SUBSTRATE | WHAT_IS_MISSING | KNOWN_INTERACTIONS | FAIL_CLOSED_BOUNDARY | RECOMMENDED_OWNER | RECOMMENDED_MILESTONE | NOTES |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| E-01 | Postcombat main and priority | CR 505, 117 | Main-phase priority/action window after combat. | R / R / R | PARTIAL | MERGED_MASTER | TurnPosition/S1 boundary; turn-structure, basic-priority specified | PostcombatMain position. | Generic Priority DTO only. | No BasicPriority event flow. | Turn position only. | Pass/action pipeline. | Combat cleanup must precede priority. | S1 downstream boundary rejects. | turn / priority | M3 | — |
| E-02 | End step and beginning-of-end-step triggers | CR 513.1–513.2, 603 | Trigger and priority window; delayed ability timing does not back up. | R / R / U | PARTIAL | MERGED_MASTER | Position + placeholder trigger state; turn-structure | EndStep, TriggerRecord/EffectRecord placeholders. | No trigger choices. | No trigger detection/ordering/stack placement. | No trigger view. | Trigger work and priority sequence. | Unsupported nonempty triggers fail closed. | SBA before priority × trigger detection/stack placement × APNAP ordering. | trigger / turn | M3/M4 | R1 trigger need pending card review. |
| E-03 | Cleanup hand-size action | CR 514.1 | Active player discards to max hand size without stack; player chooses cards. | D / R / U | MISSING | ABSENT | cleanup-reset specified, not discard | Hand zone exists; no maximum-hand count/turn config. | ChooseMany could represent discards; no legal producer. | No discard event/cursor/zone transition flow. | Hand list projection absent in current Magic codec. | Hand count limit, player-selected cards, simultaneous/ordered discards and S2 application. | M3 S1 only admits hand already within limit. | Private hand choice × zone transitions × retained knowledge × replay/checkpoint. | turn / zones | M4-Foundation | R1 card pool/hand growth pending review. |
| E-04 | Cleanup damage/duration reset | CR 514.2, 120.6 | Remove all marked damage simultaneously; expire end-of-turn/this-turn effects. | R / R / U | PARTIAL | DESIGN_ONLY | Mark storage only; cleanup-reset specified | `marked_damage` field exists. | No choice in ordinary case. | No reset event/delta/cursor producer. | Marks are not projected by Magic M3. | Simultaneous reset and effect-duration lifecycle. | S1 cleanup requires quiescent/no-marked-damage state. | Fail closed if selected reset profile incomplete. | cleanup / damage | M3 | First M3 scenario says damage persists until cleanup. |
| E-05 | Cleanup repeat exception | CR 514.3a, 704, 603 | If SBA or triggers remain after cleanup actions, repeat cleanup after stack/priority. | D / D / U | MISSING | ABSENT | cleanup-reset specified only quiescent path | Placeholder effects/triggers only. | No cleanup continuation. | No repeat/fixed-point/trigger cursor. | No repeated cleanup view. | Trigger and SBA repeat scheduling until stable. | Initial Foundation excludes triggers/cleanup discard; only ordinary quiescent cleanup accepted. | No implicit second cleanup loop. | turn / priority / triggers | Later M4 | R1 interaction unknown. |
| E-06 | Handoff to next turn | CR 500.1, 512–514 | End cleanup, advance turn/active player and enter next Untap. | R / R / R | PARTIAL | MERGED_MASTER | S1 cleanup transition; turn-structure | turn number, active player, position. | No intervening priority products implemented. | TurnNumberChanged/ActivePlayerChanged/TurnPositionChanged S1 product. | Temporal observation fields exist. | Compose next turn with priority/draw and nonempty Magic state. | Current S1 accepts only a narrow quiescent state. | Full turn response path stops at unsupported windows. | turn | M3 | S1 boundary is covered; full cycle is not. |

### Game outcome

| FLOW_ID | FLOW_POSITION | CR_AUTHORITY | RULE_MEANING | A / B / C | CURRENT_STATUS | IMPLEMENTATION_LOCATION | EXISTING_CAPABILITY / CODE_LOCATION | EXISTING_STATE_SUBSTRATE | EXISTING_DECISION_SUBSTRATE | EVENT_DELTA_SUBSTRATE | OBSERVATION_SUBSTRATE | WHAT_IS_MISSING | KNOWN_INTERACTIONS | FAIL_CLOSED_BOUNDARY | RECOMMENDED_OWNER | RECOMMENDED_MILESTONE | NOTES |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| OUT-01 | Win/loss/draw/concession | CR 104.1–104.4, 119.6, 704.5a | End game at winning/loss condition; life-loss SBA and simultaneous draw. | R / R / R | PARTIAL | BRANCH_CANDIDATE | EpisodeStatus/PlayerState; S3.A tests only; SBA specified | life, has_lost, TerminalReason/PlayerOutcome. | No concession action; response protocol only. | LifeChanged exists; no PlayerLoses/StateBasedActionsApplied event. | Terminal status is separate from InformationState. | Authoritative terminal rules, typed loss audit, concession and simultaneous outcomes. | Loss must be same simultaneous batch with creature moves. | Contract rejects unexplained has_lost mutation. | SBA / game end | M3 | Task 9B0 design only; terminal producer RED. |

### Cross-cutting substrate and support

| FLOW_ID | FLOW_POSITION | CR_AUTHORITY | RULE_MEANING | A / B / C | CURRENT_STATUS | IMPLEMENTATION_LOCATION | EXISTING_CAPABILITY / CODE_LOCATION | EXISTING_STATE_SUBSTRATE | EXISTING_DECISION_SUBSTRATE | EVENT_DELTA_SUBSTRATE | OBSERVATION_SUBSTRATE | WHAT_IS_MISSING | KNOWN_INTERACTIONS | FAIL_CLOSED_BOUNDARY | RECOMMENDED_OWNER | RECOMMENDED_MILESTONE | NOTES |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| X-01 | Complete state, object/card identities and allocators | CR 108–110, 400.1–400.7, 700.4 | Represent every semantic value that changes legality/outcome/visibility; zone move creates new object. | R / R / R | HAVE | MERGED_MASTER | `mtgml-state`; state/identity substrate | EngineState, GameObjectId/PhysicalCardId, owners/controllers, locations/order, RNG/allocators, knowledge. | Bound identity resolver. | Full StateDelta and digest. | Opaque identity/knowledge model. | Card-type/rules/land/mana/spell/ability state for playable scope. | Zone, LKI, hidden info, replay and card semantics. | Unsupported state fields/profile fail structural or rules validation. | state / zones_identity | M3 foundation | This is robust generic substrate, not full Magic state. |
| X-02 | Zone transitions/incarnation | CR 400.1–400.7, 401–404, 700.4 | One selected move creates NEW GameObjectId and exact old/new snapshots/order/lifecycle. | R / R / R | PARTIAL | MERGED_MASTER | `zone_incarnation.rs`; Task 9A workspace branch-reviewed; Z | Objects, zones, ordered top offsets, physical identity, lifecycle. | Direct selected request is internal testkit, not player Decision. | ZoneTransition and PerspectiveOccurrence cursor/delta. | Perspective life cycle supports opaque remap/private acquire. | Draw/SBA producers; multi-move batch contract and verified transition replay for coverage. | SBA loss, Draw, cast/resolve all depend on exact zone composition. | Standalone moves remain exactly two families; reject unsupported sources/references. | zones_identity | M3, then M4 | Registry `implemented`, `covered` blocked by authoritative replay. |
| X-03 | Decision protocol and actor binding | CR 101.4, 115–117, 508–509, 601 | Every player choice is explicit, legal, complete, actor-bound and canonical. | R / R / R | HAVE | MERGED_MASTER | `mtgml-decision`; generic Decision V2 | DecisionId, PlayerDecisionId, CandidateId, opaque↔trusted bindings. | Closed ChooseOne/Many/Number/Order and response V2. | Decision create/clear event cursor. | Projected actor request only. | Magic legal candidate generation for lands, spells, targets, mana and combat. | Multi-step choices compose with typed continuation. | No default/implicit/random answer; missing legal choice fails closed. | decision + semantic owner | M2/M3 | `CastSpell`/`PassPriority` intents existing are not producers. |
| X-04 | Continuations and staged choices | CR 601/choice-specific; architecture contract | Persist complete resumable player-choice state, reject atomically, retire on completion. | R / R / R | PARTIAL | BRANCH_REVIEWED | `mtgml-state` payload; Magic SBA Task6/7 branch; generic continuation | SyntheticM2Assembly and MagicSbaGraveyardOrderV1. | Reuses Order/Cardinality V2. | SbaGraveyardOrderChosen cursor. | APNAP progress projected via Magic M3 codec. | Casting/target/trigger/resolution continuations and S3 production restore/fork. | Final order cannot persist in all-owners-complete state. | No callbacks/local stage cache. | execution / rules owner | M3/M4 | S1 production restore rejects SBA continuation; no prod S3 ID. |
| X-05 | Authoritative events, exact delta and cursor | Engine contract; CR events per flow | Every rule-relevant mutation has sequential event/cursor authority and exact complete delta. | R / R / R | PARTIAL | BRANCH_REVIEWED | rules events/contract/cursor; Task7 events reviewed; generic audit substrate | Cursor covers life, objects, turn, combat, decisions, lifecycle, S2, Order. | — | Zone, Life, taps, Decision, APNAP Order; no batch/damage/cast events. | Observed event V2 closed subset. | StateBasedActionsApplied, combat damage, priority, cast/stack, trigger/replacement cursors. | Each new producer must preserve event→delta→after-state parity. | Existing has_lost and combat changes fail unexplained mutation check. | rules semantic owners | M3/M4 | Design rows are not event implementations. |
| X-06 | Perspective observations/information safety | CR 101.4b, zones 400–406; information contracts | Project authorized public/private current state and retained knowledge without trusted identities. | R / R / R | PARTIAL | BRANCH_REVIEWED | observation Rust/Python; APNAP codec Task8; info/visibility substrate | Knowledge, opaque identities, per-player VisibleSequence. | Actor-only Decision projection. | ObservedEventEnvelope V2 lifecycle events; no fabricated APNAP event. | Synthetic M3 + Magic M3 payload, but shared production caller remains synthetic. | Full hand/card/type/action surface for real playable Magic and production S3 projection selection. | Hidden card/allocator differences must not alter unauthorized bytes. | Missing opaque mapping fails closed; no other-player opaque/trusted IDs. | information owner | M3/M4 | Task8 provides only public order progress. |
| X-07 | Environment response transaction/forced progress | Execution model; CR priority boundaries | One real response, candidate transition, optional one forced advance, projections and commit are atomic. | R / R / R | PARTIAL | MERGED_MASTER | `response_transaction.rs`, S3.0; shared response transaction | Candidate EngineState/checkpoint, counters, replay recorder. | Decision V2 actor input. | One ReplayStepV6 per real response; after-state delta. | Candidate projections validated before commit. | Terminal-vs-running and future forced closure across priority/draw; environment must not loop. | At most one kernel advance; unsupported BasicPriority is not swallowed. | One accepted response/ReplayStep × at most one forced advance × no-choice stop boundary. | environment transaction owner | M3/S3.B | Nonterminal final Order closure is blocked until S3.B/disposition. |
| X-08 | Checkpoint/restore/fork/replay | Persistence/replay contracts; CR input identities | Resume same semantic program and identities, reproduce same next Decision/products. | R / R / R | PARTIAL | MERGED_MASTER | Environment V6, Replay V6; checkpoint/replay | FullStateDigestV5, EnvironmentCheckpointV6, status/counters. | DecisionResponseV2 replay input. | ReplayStepV6 records a real response only. | Current observation payload codec bound in V6. | Production S3 semantic contract, final SBA/priority/draw backend replay witnesses; direct S2 replay remains deferred. | Continuation must restore identical APNAP Decision/order. | No event-as-input, fake response, or silent V5 identity reinterpretation. | persistence / environment | M3 | Production S3 SemanticContractId not allocated. |
| X-09 | Deterministic RNG and allocator substrate | CR 103.3/ random operations; RNG contract | Reproducible randomness and checked unique identity allocation. | R / R / R | HAVE | MERGED_MASTER | mtgml-random, mtgml-state; deterministic services | HMAC counter streams and allocator families. | Decision/opaque ID allocators. | Cursor/allocator state in exact delta/digest. | Hidden RNG/cursor not player-visible. | Game-start library shuffle algorithm/stream ownership and provenance. | Fail checked exhaustion; never infer shuffle from sorted input. | Starting-game shuffle × RNG stream/provenance × hidden library-order observation. | random / state | M3/M4 | RNG capability exists; Magic shuffle producer does not. |
| X-10 | Capability identity/lifecycle and ownership | Accepted ADRs / Foundation V2 | One stable versioned semantic owner and dependency closure per supported capability. | R / R / R | HAVE | MERGED_MASTER | registry/schema/validator and ADRs; 11 Foundation identities | no runtime dispatch key. | — | lifecycle references test evidence. | info risk classification. | Evidence for nine specified Foundation nodes and S3 closure review. | Registry lifecycle is not support proof. | No in-place semantic identity reinterpretation. | capability owners | M3/M4 | Current registry: 9 specified, 1 implemented, 1 covered, 0 certified. |
| X-11 | Card IR and executable content model | CR 108–113, 202, 205 | Typed/versioned definitions request semantics; RulesKernel determines legality/execution. | D / R / U | PARTIAL | DESIGN_ONLY | experimental `mtgml-card-ir`; no frozen card capability | CardDefinitionId only in EngineState. | no dynamic rules decision producer. | No accepted executable effect representation. | known-definition field can represent identity only. | Freeze minimal stable card/type/cost/ability/effect vocabulary from selected content. | Do not treat experimental variants or CardDefinitionId as support. | Unknown capability at load/execute fails bundle preflight. | cards/content + rules | M4-Foundation/M4-R1 | `ExperimentalEffect` explicitly unstable. |
| X-12 | Real Oracle/card/deck content | CR 100.2, 108, 202–205 | Load exact real deck card definitions and source provenance. | D / R / R | MISSING | RESEARCH_ONLY | R1 census only in archived research; `cards/` has examples; none registered for cards | example manifests/decks, no R1 definitions. | no deck/game endpoint. | no card-program events. | no real card face/text projector. | Acquire/validate immutable Oracle/card source, definitions, deck lists, generated references and bundle. | Card data/source rights are separate; no arbitrary sourced list becomes authority. | Unknown/malformed content cannot run. | card IR/content | M4/M4-R1 | R1 source package lists R1_full_list.txt internally but archive/list isn't a tracked working-tree deck file. |
| X-13 | Format/deck legality | CR 100.2a/100.6 plus format/tournament authority | Validate deck construction and selected Standard policy where claimed. | D / D / R | MISSING | RESEARCH_ONLY | R1 Standard research summary only; no format capability in 11 | FormatState::None or unused Commander fields. | no deck legality request. | no legality event. | no legal-deck status projection. | Pin Standard rotation/banlist/tournament policies and exact deck legality profile. | R1 research checked Standard framing but is advisory, not runtime policy. | Reject unknown or stale format identity. | format / deck legality | M4-R1 | First playable B may use fixed trusted custom lists without a Standard support claim. |
| X-14 | Locked bundle certification/benchmark | Certification contract | Support claim binds exact engine/content/capability/snapshot/evidence bundle. | D / D / R | PARTIAL | MERGED_MASTER | cert tooling + example bundle only; closure validator | schema-shaped manifests/example. | — | gate runner references. | information-risk closure. | Real R1 bundle and runtime semantic/property/replay/benchmark gates. | R1×R1 remains provisional reference/throughput candidate, not frozen benchmark. | No “supports deck/card” claim from parse/count/compile. | certification / benchmark | M4-R1 | Research observed 69 capability closure, 0 reviewed obligations, 0 satisfied evidence; not an implementation queue. |
| X-15 | Multiplayer and Commander | CR 800–811, 903 | Multi-player turn/APNAP/priority and Commander-specific state/cost/damage. | D / D / D | DEFER | MERGED_MASTER | structural Commander helpers only; no Commander capability in M3 closure | EngineState player map; optional CommanderState ledgers. | generic PlayerId/Decision substrate. | no multiplayer/Commander rule events. | no format-specific projector. | New multiplayer/APNAP/deck/format semantic scope and reviewed versioned capabilities. | APNAP currently only two-player active then nonactive. | Require exact admitted two-player `FormatState::None` for M3. | format / multiplayer | Later | `mtgml-commander` helpers do not certify Commander. |
| X-16 | Advanced keywords, tokens, attachment, copy, variants | CR 111, 122–123, 613–616, 702–731, 900+ | Optional mechanics alter legality, state, characteristics, identity or outcomes. | D / D / U | DEFER | ABSENT | no M3 production capability; none for broad family | token physical-card absence is rejected in S3A; no counters/attachments/layers. | no such decisions. | no families. | only currently declared state. | Review only those mechanics required by the exact selected card pool. | R1 card interactions must be derived per exact Oracle text; don't adopt the research census wholesale. | No heuristic approximation. | mechanic owner | M4-R1/Later | Future cards may force selective promotion from DEFER. |
| X-17 | Search, training, reward, optimized rollout | ML/experiment contracts | Policy/search/data generation is downstream of authoritative legal transitions. | D / D / D | DEFER | RESEARCH_ONLY | no production trainer/search/runtime; none | replay/checkpoint can later underpin datasets. | no rewards/training trajectory contract. | no ML action semantics. | observations are not trajectories/reward APIs. | M5 training, determinization, dataset and optimized backend designs. | Don't duplicate legality in Python or benchmark drivers. | No training support claim. | ML/search | M5/Later | Outside M3, basic playable and R1 semantic closures. |

## 4. Existing 11-capability audit

The lifecycle is read directly from [the registry](../cards/capabilities/registry.json).
Registry paths are references, not proof. `required remaining` is for each
capability's declared M3 target scope.

| Capability | Registry lifecycle now | Implementation location / actual code | State readiness | Decision readiness | Event/delta readiness | Observation readiness | Replay/checkpoint readiness | Required remaining and interaction |
|---|---|---|---|---|---|---|---|---|
| `turn-structure@0.1.0` | covered | `MERGED_MASTER`: `turn_structure.rs`, Magic S1, contract/cursor, Reference | temporal/active/turn/tap/control facts enough for S1 | no decision for untap; priority downstream missing | S1 turn and untap event/delta cursor | S1 synthetic M3 turn/priority fields | V6 under S1 identity; S1 exact tests | M3 needs composed passes/draw/combat/cleanup flow while preserving S1 semantics. |
| `basic-priority@0.1.0` | specified | `DESIGN_ONLY`: Foundation V2 and Tasks 11–14 | PriorityState exists but no kernel action flow | PassPriority intent/binding shape exists; no producer | no PriorityChanged event/delta/cursor | priority kind can be projected; not a real window | no production priority replay | implement pass-only window after SBA, preserve actor/pass count, then environment/replay integration. |
| `draw-card@0.1.0` | specified | `DESIGN_ONLY` producer; uses merged S2 seam | Library/Hand/order and identities exist | no draw response/decision; normal draw itself is turn-based | S2 ZoneTransition event exists; no Draw context/cursor | private owner-knowledge path exists; no Draw producer | not backend replay-witnessed | Draw×S2×SBA×priority, empty library excluded by current M3 scope. |
| `cleanup-reset@0.1.0` | specified | `DESIGN_ONLY`; S1 only admits quiescent cleanup | marked_damage field exists | none in bounded no-discard case | no damage-reset producer/event | no Magic mark surface | no cleanup-reset producer replay | implement/reset only after S3 selected damage flow; initial M3 requires safe cleanup case. |
| `combat-damage@0.1.0` | specified | `DESIGN_ONLY` | CombatState/source marks/power exist | bounded no-choice assignment target, no producer | no assignment/damage semantic families | combat assignment not projected | no producer/replay | normal simultaneous assignment and post-damage SBA; no first/double strike etc. |
| `combat-phase@0.1.0` | specified | `DESIGN_ONLY`; Position enum only | closed five-step combat enum, optional CombatState | none | no combat progression event cursor | temporal position only | no progression replay | implement combat step/skip/priority coordination. |
| `declare-attackers@0.1.0` | specified | `DESIGN_ONLY` | attackers vector, tapped/controller/control history | ChooseMany can represent but no legal producer | no atomic declare event/delta | generic opaque IDs only | no producer replay | eligible set, decision soundness/completeness, tap/attack event. |
| `declare-blockers@0.1.0` | specified | `DESIGN_ONLY` | attacker→zero/one blocker map | ChooseOne/Confirm shape can represent bounded assignment | no producer/cursor | generic opaque IDs only | no producer replay | derive choices; multi-blocker fails closed; later combat/SBA interaction. |
| `damage-and-life@0.1.0` | specified | `DESIGN_ONLY` producer; state facts exist | life, has_lost, marks and simple sources | no assignment/action choice in selected slice | LifeChanged event exists but no source-bound damage producer/cursor | life public products available; source attribution absent | no damage producer replay | damage result/source/recipient evidence and cleanup/SBA interaction. |
| `state-based-actions-combat@0.1.0` | specified | `BRANCH_CANDIDATE`: Rules derivation/continuation validator, APNAP stage and 9B0 tests/design | life/has_lost/simple creatures/marks/CombatState | Order V2, continuation and APNAP are implemented candidate | SbaGraveyardOrderChosen exists; StateBasedActionsApplied not implemented | Magic APNAP public progress codec branch reviewed | no production S3 contract or SBA batch replay | Task9B atomic batch/fixed point/terminal, Task10 restore/fork/replay. No-order combat pre-boundary REDs at exact inventory HEAD. |
| `zone-incarnation@0.1.0` | implemented | `MERGED_MASTER`: sole one-move S2 authority; `BRANCH_REVIEWED`: workspace composer from Task9A | two selected physical move families, new GameObjectId/LKI/order | direct testkit request; not a player action producer | ZoneTransition + PerspectiveOccurrence and cursor/contract | remap/private acquire semantics | direct S2 move not replay-covered; lifecycle intentionally blocks `covered` | Task9 ordered multi-move batch must reuse same workspace, then independent S2 replay/coverage review. |

**Lifecycle reconciliation:** the 11 identity rows match the current
registry and README/ROADMAP summary (9 `specified`, 1 `implemented`, 1
`covered`, 0 `certified`). S1 is covered only within its narrow profile; S2
is implemented but not covered. No lifecycle change is made by this inventory.

## 5. Reuse audit

| Missing gameplay work | Reusable current primitive | What it does not supply |
|---|---|---|
| Starting hand / mulligan / target or ordering choices | Decision V2, candidate bindings, opaque IDs and continuation payloads | Rules-derived candidate legality, simultaneous mulligan rounds and source-specific choice semantics. |
| Land/creature/spell zone movement | S2 `apply_selected_zone_transition_in_workspace` | Land legality, spell casting/resolution, generic destinations, or support for arbitrary zone families. |
| Spell stack / response | Decision V2, EngineState stack slots, shared response transaction | Stack mutation/resolution semantics, priority windows and card programs. |
| Triggers or trigger ordering | typed continuation, APNAP/Order Decision, event cursor | Trigger detection, event/source binding, queue placement or ability resolution. |
| Multi-object SBA | S2 workspace, S3.A action plan, continuation/order, exact delta | `StateBasedActionsApplied`, complete cursor contract, loss/combat closure and batch validation. |
| Player-safe card/game views | knowledge lifecycle, opaque IDs, InformationStateV2/PlayerStepV2, Magic M3 codec | Real card visibility/hand observations and production S3 codec selection. |
| Determinism/restart | RNG, allocators, V6 checkpoint/replay, fork | Magic game-start/shuffle semantics, production S3 identity and replayable producers. |
| Card/content support | capability registry, experimental Card IR, example bundles/certifier | Stable Card IR, primary Oracle set, card definitions, exact deck closure and semantic executors. |

Do not create a new universal `RulesCase` protocol, runtime plugin registry,
card handler dispatch, or second rules engine. Add typed owner-local semantics
only when a selected flow requires them. The capability graph determines
semantic ownership/reuse; the chronological flow determines the order in
which those owners are implemented.

## 6. Overengineering risks

| Candidate expansion | NEEDED_NOW | Reason |
|---|---|---|
| General CombatState with historical blocked/unblocked bit | NO for this M3 slice | Task9B0 restricts S3.A combat participant pruning to post-damage. A later pre-damage removal scope needs a separately reviewed representation. |
| Universal layer/dependency engine | NO for M3 or first minimal playable pool | Foundation uses simple base characteristics and fails closed; R1 exact card review must establish whether a smaller specific reusable subset suffices. |
| General replacement/prevention interpreter | NO for M3/minimal vanilla pool | Selected combat is unmodified; don't prebuild without card/interaction evidence. |
| All SBA families at once | NO | Current M3 selects life loss, zero toughness and lethal marked damage only. Expand only for selected card/game requirements. |
| Full multiplayer/Commander rules | NO | Horizons A/B/R1 are explicitly two-player and non-Commander. |
| Generic public Magic flow plugin/handler registry | NO | Architecture says card definitions request typed semantics, and the Rust kernel owns execution. |
| New Decision framework for each mechanic | NO | Reuse Decision V2 + validated continuations; add new domain only after a concrete requirement demonstrates need. |
| Generic replay/event version bump | NO | V6 stores real response and full state identity; current gap is production S3 identity/producer evidence, not an absent generic container. |
| Native/optimized second backend before parity | NO | Current architecture calls for a reference backend and exact parity before optimization. |

## 7. Explicit horizons and plan

### M3 critical path

`M3_CRITICAL_PATH` from this HEAD is seven major semantic/evidence blocks:

1. **S3.A atomic batch:** Task9B design REDs → atomic one-round producer,
   `StateBasedActionsApplied`, exact S2 move composition, loss/terminal and
   bounded post-damage combat closure. Task9B0 exact-head review precedes this.
2. **S3.A durability:** fixed point, restore/fork at APNAP stages, production
   S3 semantic contract, replay and all atomicity evidence. Disposition the
   known nonterminal response/Basic Priority barrier before claiming closure.
3. **Basic Priority Rules:** event/state/pass contract and pass-only semantic
   implementation.
4. **Magic response integration:** admit reviewed S3 identity, wire Reference
   player responses to the shared response transaction, verify rejection and
   actor-only behavior.
5. **Draw disposition/producer:** freeze event choice, RED and implement
   ordinary Draw through S2.
6. **Normal-turn composition:** Upkeep pass → Draw → priority; verify combat,
   cleanup, V6 replay and S2 interactions/coverage.
7. **M3 closure:** lifecycle/interaction evidence, workspace and full exact-head
   gates; no promotion from candidate or implementation alone.

```text
ESTIMATED_MAJOR_SEMANTIC_BLOCKS_REMAINING_M3 = 7
```

This is grouped from the current accepted plan's Task9B/10, Tasks11–14,
Tasks15–21 and Tasks22–24. Task 9B0 and Task 9B must not be collapsed: the
former freezes the design and RED; the latter owns production semantics.

### First playable critical path

`FIRST_PLAYABLE_CRITICAL_PATH` is five major blocks **after M3**:

1. **Pregame/deck initialization:** declared 60-card deck inputs, legal player
   configuration, starting player, shuffle, initial life, opening hand,
   mulligan and first-turn skip, all with V6 provenance/restart behavior.
2. **Minimum real card model/content:** freeze the smallest Card IR/source
   contract and exact initial vanilla/basic spell content; keep card-definition
   facts distinct from reusable Rules semantics.
3. **Land and mana action flow:** land classification/per-turn allowance,
   mana pool, land/mana abilities and atomic costs/payment.
4. **Casting/stack/target/resolution flow:** cast legality, selected costs and
   targets as required by the chosen content, stack placement, pass/response,
   resolution and destination lifecycle. This can remain a narrow card pool;
   it cannot claim arbitrary spell semantics.
5. **Playable closure:** fixed deck lists, legal response completeness,
   information safety, state/event/delta/replay/checkpoint/fork, full-game
   conformance and an explicitly scoped playable claim.

```text
ESTIMATED_MAJOR_SEMANTIC_BLOCKS_REMAINING_PLAYABLE = 5
```

These blocks assume the implementation reuses M3 Decision, transaction,
continuation, S2, SBA, priority and V6 machinery. They do not include the seven
M3 blocks a playable engine depends on.

### R1 critical path

`R1_CRITICAL_PATH` adds three provisional blocks after the playable shell:

1. **R1 source/format/content closure:** acquire immutable Oracle/source,
   exact archived R1 list, current Standard/banlist policy and a locked card/
   deck bundle.
2. **R1-specific reusable semantics:** review each card's exact Oracle text;
   then implement only uncovered target, burn/damage-source/history, Ojer
   damage-modification/transformation/return, trigger or other interactions.
3. **R1 mirror evidence:** exact legal 60-card mirror, full-game conformance,
   hidden-information/replay parity, bundle certification and reference
   benchmark identity.

```text
ESTIMATED_MAJOR_SEMANTIC_BLOCKS_ADDED_BY_R1 = 3 (PROVISIONAL)
```

The count is architecture-level, not a statement that exactly three
capabilities or work items suffice. The R1 deck list/card closure is not
available as a tracked file at this HEAD; the Standard research's `69`
candidate closure is not used to derive the count or implementation queue.

## 8. What to defer explicitly

| Feature | Horizon disposition |
|---|---|
| Pregame/start procedure, shuffle, opening hand and mulligan | `DEFER` for A; required B/C. |
| Land play, mana, casting, stack, costs and resolution | `DEFER` for A; B only at the first reviewed content scope; C card review may extend. |
| Generic targets/modes | `DEFER` for A; required in B/C when selected card pool contains such choices. |
| General triggers, replacements/prevention, layers/copy | `DEFER` for A and minimal vanilla B; `UNKNOWN_PENDING_CARD_REVIEW` for C. |
| Commander, multiplayer, casual variants, alternate turn structures | `DEFER` across A/B/C as currently defined. |
| Full SBA catalogue, token/legend/Aura/Equipment/counter cases | `DEFER` unless a later selected card/content scope requires a specifically reviewed subset. |
| Arbitrary keyword/card coverage | `DEFER`; only exact locked card closure can create requirements. |
| Search/determinization/training/rewards and optimized rollout backend | `DEFER` to M5/later. |
| Deck/certification/benchmark claims | `DEFER` until exact content, policy identity and all evidence gates exist. |

## 9. Early blockers and repository alignment audit

### Early blockers surfaced

1. **M3 no-order Combat path:** Task 9B0 FIX-01 now has RED evidence that a
   fresh no-order round with a dying CombatState participant at
   BeginningOfCombat, DeclareAttackers, DeclareBlockers or EndOfCombat is
   currently accepted by Rules support validation; a separate CombatDamage
   no-order forced path stops at `UnsupportedStagePath`. Task 9B must gate
   both response and forced-progress entry paths consistently.
2. **Nonterminal final Order environment barrier:** environment response
   transaction forces once when accepted/Running/no Decision. Current kernel
   then fails at BasicPriority. Task9B0 records this as
   `BLOCKED_UNTIL_S3_B_OR_REVIEWED_TRANSACTION_DISPOSITION`; do not claim
   nonterminal final-order production replay closure before disposition.
3. **S3 production admission:** production catalog has the historical S1
   closure only; Task8 codec and S3.A conformance profile do not create an
   admitted production S3 SemanticContractId. Restore/fork remains deferred.
4. **S2 covered evidence:** S2 implementation is present, but the direct S2
   transition has no accepted response replay input; registry deliberately
   says implemented/not covered pending a real producer/replay witness.
5. **M4 content source:** real Card IR, Oracle definitions and an R1 deck list
   are absent from tracked production content; OD-006 remains open for source/
   distribution basis. The research archive is not a runtime card catalog.

### Cross-layer contradiction search

```text
CROSS_LAYER_CONTRADICTIONS = 0 DIRECT CONTRACT CONTRADICTIONS FOUND
```

Checked current registry, Foundation V2, S3 plan, README/ROADMAP, Rust state/
kernel/decision/event/cursor/environment/replay, Python codecs, schemas and
research declarations. Status differences are deliberate evidence levels,
not contradictory behavior: S1 is covered, S2 implemented/not covered, nine
Foundation capabilities remain specified, S3.A is branch-only, and S3B/C are
not authorized. The Task10-before-S3B replay requirement versus the 9B0
nonterminal response barrier is an explicit **sequencing dependency requiring
disposition**, not silently claimed resolved. R1's archived list/census is
explicitly research-only, and experimental Card IR is labeled unstable.

The 31 registry S2 case IDs versus 39 S2 Rust conformance functions are
different counting units (case identifiers vs test functions), not by
themselves a scope contradiction. The branch's 9B0 test REDs are not counted
as implemented behavior.

## 10. Required executive closure values

```text
CURRENT_HEAD = 8cafb91b121cc2e29f3837fffbd8a189cbf817
MASTER = 66f3b713787cad89674257f6e0b6448b9fd568f9
FLOW_ROWS = 56
M3_REQUIRED_ROWS = 30
PLAYABLE_REQUIRED_ROWS = 47
R1_REQUIRED_OR_PENDING_ROWS = 54

HAVE = 5
PARTIAL = 30
MISSING = 18
DEFER = 3

M3_MAJOR_SEMANTIC_BLOCKS_REMAINING = 7
PLAYABLE_MAJOR_SEMANTIC_BLOCKS_ADDED = 5
R1_ADDITIONAL_MAJOR_SEMANTIC_BLOCKS = 3_PROVISIONAL
CROSS_LAYER_CONTRADICTIONS = 0_DIRECT
NEW_EARLY_BLOCKERS_FOUND = 5

M3_CRITICAL_PATH =
  S3.A atomic batch
  -> S3.A fixed point/replay/production identity
  -> Basic Priority Rules
  -> Reference/environment response integration
  -> Draw through S2
  -> normal-turn composition and S2 coverage
  -> cumulative M3 closure

FIRST_PLAYABLE_CRITICAL_PATH =
  pregame/decks
  -> minimum card model/content
  -> land/mana
  -> cast/cost/target/stack/resolution
  -> full-game evidence/certification for named scope

R1_CRITICAL_PATH =
  exact R1/source/Standard closure
  -> card-by-card missing-semantic review
  -> selective reusable R1 semantics
  -> R1 mirror evidence/certification

RESEARCH_ONLY = YES
DOES_NOT_CHANGE_CAPABILITY_LIFECYCLE = YES
DOES_NOT_AUTHORIZE_IMPLEMENTATION = YES
```

The capability architecture is retained: capabilities decide semantic
ownership and reuse; the actual Comprehensive Rules flow decides development
order. This inventory proposes larger work blocks, not new capability
identities or implementation authorization.
