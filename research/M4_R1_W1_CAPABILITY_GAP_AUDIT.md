# R1 × W1 Capability and Turn-Structure Gap Audit

**Audit type:** repository/research only; no implementation or test changes.

**Manafold baseline:** `d066916d71f4bd35ee447a96c049aad7961da40d` (verified `origin/master`).

**Forge source:** `17c1ba92149b84127749bf84c231ed75107df1a2`.

**Rules authority snapshot:** WotC Comprehensive Rules effective 2026-09-25, source ID `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`; Oracle cross-check: Scryfall bulk snapshot `oracle-cards-20260925210158.jsonl.gz`, SHA-256 `c607300fe03ce0d9f59181b1bb33d8001e2fa339b8c68eefd70a6501fa757623`.

## 1. Verified repository baseline

Fetched `origin/master` and inspected it at the exact SHA above. PRs #257, #258 and #259 are merged. The current branch is based on that commit. Issue #222 is the locked 60-card-per-deck scope; aliases are `A Most Helpful Weaver` → `Origin of Spider-Man` and `Wonderweave Aerialist` → `Skyward Spider`. Issue #237 is coordination/research input, not an executable contract or current capability inventory; its 92/74 counts were reconciled against an older commit and must not be treated as current.

The only R1/W1 artifacts are 26 validation-only candidates under `tools/forge-card-script-parser/candidates/r1w1/`, built from 26 Forge scripts and containing 27 ordered faces (Ojer Axonil and Temple of Power are one definition). The 26 candidates have `UnprofiledV1`, no ability identities or capability requirements, and are not admitted production cards. Only Mountain and Plains have current executable profile coverage, through the existing `basic-land@1.0.0` definition/profile. There are no locked R1/W1 deck manifests in `cards/decks/`; the examples there are unrelated.

The candidate report records 96 preserved Forge constructs (`A:12`, `T:14`, `R:3`, `S:8`, `K:21`, `SVar:38`). These are source entries, not 96 distinct mechanics. All 26 Forge scripts parsed and lowered base characteristics; this proves neither Oracle authority nor gameplay support. Cross-checking their 27 face names, mana costs, type lines and P/T against canonical non-art-series Oracle records found no mismatches. Forge `Oracle:` prose differs on 11 faces only in self-name substitutions and/or omitted reminder text; the comparison does not validate script semantics.

## 2. Executive summary

Manafold has a useful deterministic state, event, decision, replay and Card IR substrate, plus two different narrow execution areas: M3/Foundation V2 bounded turn/combat rules, and the production M4.2 Basic Land profile. Neither provides a complete game for these decks. M3's ordinary priority path has an empty action surface and its bounded combat/SBA assumptions reject the Aura, token, effect and stack states these lists can reach. The production Basic Land path does not construct a game or play through turns.

The accepted M4 Shared Foundation provides a dependency order and typed V8 data contracts, not the missing semantic producers. G0 is merged. S1-A/B code exists, but their spec and plan still say proposed and “Implementation authorized: NO” while PR #255/#256 merged their code; reconcile that governance status before authorizing S1-C or claiming the accepted S1 scope complete. S2 through S6b have no RulesKernel execution implementation on this baseline; S7 is integration/conformance evidence, not a capability.

The minimum complete matchup cannot be obtained by implementing an isolated card executor. It needs ordinary game construction, generic costs/casting/targets/stack/priority, derived characteristics and attachments/effects, trigger and replacement processing, damage/counters/zones/SBAs, ordinary combat/turn rules, cleanup and deterministic replay/observation integration. Reachable late dependencies include the legend rule (Ojer and Origin), Role uniqueness, tokens, Saga lore, and Ward. Do not omit them merely because they were deferred from the first shared cut.

Audit vocabulary below is descriptive only: `EXISTS_AS_STATE` means persisted data exists; `EXISTS_AS_TYPED_CONTRACT` means an API/schema exists; `EXISTS_AS_IMPLEMENTATION` means relevant code exists; `EXECUTABLE_IN_PRODUCTION` requires a reachable production rules path; `COVERED_FOR_BOUNDED_SCOPE` means tests/contracts establish only the named bounded scope; `COVERED_FOR_REQUIRED_R1_W1_SCOPE` would require the entire reachable matchup closure (currently none); `NOT_IMPLEMENTED`, `BLOCKED`, and `UNKNOWN` are used literally. State fields and enum variants alone never count as implemented behavior.

```text
REGISTERED_CAPABILITIES = 14
COVERED_BOUNDED_CAPABILITIES = 11
SPECIFIED_CAPABILITIES = 3
EXISTING_EXECUTABLE_CARD_PROFILES = 1 profile family (Basic Land; Mountain and Plains)
R1_DEFINITIONS = 12
W1_DEFINITIONS = 14

SHARED_RULE_GAPS = 15 semantic work areas (S1 completion through S6b; see §7)
R1_EXCLUSIVE_GAPS = 2 (Warp permission; Ojer transformation/legend interaction)
W1_EXCLUSIVE_GAPS = 5 (Saga chapters; Role/token creation; Eerie; Descended; Aura-linked return)
GAME_SETUP_GAPS = 1 package (deck construction, shuffle/deal, mulligan, starting choices)
TURN_STRUCTURE_GAPS = 1 package (ordinary full turn and all priority/cleanup boundaries)
INTERACTION_GAPS = 4 cross-list packages (Ward/targets, Dryad replacement, Ojer damage, Aura exile)

ALREADY_IMPLEMENTED_AND_REUSABLE = bounded state/identity/zones, Card IR validation, Basic Land runtime,
  M3 empty-stack turn/combat path, deterministic persistence/replay primitives
EXISTING_BUT_REQUIRES_EXTENSION = S1-A/B query/base mapping, state payloads for mana/history/counters/
  attachments/faces/stack/effects, decision/event/delta/replay contracts, bounded M3 turn skeleton
NOT_IMPLEMENTED = generic cast/payment/stack/target actions, triggers/replacements/effects, full SBA,
  Auras/tokens/Saga/legend rule, complete setup and complete ordinary turn structure
UNKNOWN_OR_UNVERIFIED = full closed dependency and replay/observation parity for any future card profile;
  current game setup semantics because no producer exists

NEXT_RECOMMENDED_WORK_PACKAGE = reconcile S1 authority/status, then implement S1 completion as accepted
  (counter-adjusted characteristics, attachment-derived relationships/continuous query, conformance)
NEXT_WORK_PACKAGE_DEPENDENCIES = governance/status reconciliation; G0 V8 contract family already merged
```

The “gap” counts are planning buckets, not capability-registry keys and not Forge-entry counts. Shared areas are enumerated as: S1 completion; S2 cost/payment; S2b source-derived static contributions; S3 target domains; S4 casting/activation/stack; S5 trigger execution/order; S6a temporary effects/expiry; S6b replacement/counters/damage/zones; S7 integration evidence; plus six general cross-cutting producers/consumers needed to join those owners (priority/action closure, event/SBA stabilization, characteristic-to-observation, object/face transitions, replay/checkpoint integration, setup). Grouped packages above avoid pretending the inventory is a count of independent rule atoms.

## 3. Existing capability inventory

`cards/capabilities/registry.json` has 14 entries: 11 `covered` and 3 `specified`. All eleven covered entries explicitly limit the claim to bounded Foundation V2 M3 and deny general card/deck/playability support. “Covered” is not production admission for R1/W1. The three specified entries have no registry-linked conformance implementation path; the Basic Land profile's production behavior is evidenced separately and does not silently promote registry lifecycle.

| Capability key | Lifecycle and actual bounded scope | Implementation / conformance evidence | R1/W1 extension or exclusion |
|---|---|---|---|
| `rules/basic-priority` | covered; empty-stack pass progression | `crates/mtgml-rules/src/basic_priority.rs`, `magic.rs`; `tests/basic_priority.rs`, environment `tests/magic_rules_production.rs` | Add legal action generation, stack resolution, triggers and response windows; current validator rejects nonempty action surface. |
| `rules/cleanup-reset` | covered; bounded combat-mark reset/turn handoff | `magic.rs`, `turn_structure.rs`, `events.rs`; bounded cleanup tests in rules/environment | Add discard, damage clearing at correct cleanup point, duration expiration, trigger/SBA repetition and hand-size choices. |
| `rules/combat-damage` | covered; one normal damage step in constrained combat | `combat_damage.rs`, `magic.rs`; `tests/batch_e.rs`, `tests/turn_structure.rs` | First/double strike, assignment choices, multiple blockers, trample/deathtouch/lifelink and replacement/damage interactions. |
| `rules/combat-phase` | covered; explicit bounded combat phase progression | `turn_structure.rs`, `magic.rs`; turn-structure tests | Full priority/trigger/SBA windows and complete combat decisions are absent. |
| `rules/damage-and-life` | covered; bounded combat damage/life and marked damage | `magic.rs`, `combat_damage.rs`; simultaneous damage/SBA tests | Generic noncombat damage, prevention/replacement, lifelink and damage event consumers are absent. |
| `rules/declare-attackers` | covered; <=8 attackers, simple all/subset choices | `magic.rs`, turn/combat tests | Needs card-derived abilities, attack restrictions/triggers and full choices. |
| `rules/declare-blockers` | covered; at most one relevant blocker | `magic.rs`, `combat_damage.rs`; combat tests | Multiple blockers, evasion/ward interactions, restrictions, ordering and assignment decisions. |
| `rules/draw-card` | covered; one ordinary draw-step top-to-hand movement | `magic.rs`, `zone_incarnation.rs`; bounded turn tests | Game setup, empty-library loss and full turn/priority rules. |
| `rules/state-based-actions-combat` | covered; life <= 0, zero toughness, lethal marked damage in closed creature scope | `state_based_actions.rs`; `tests/state_based_actions.rs` | Legend, illegal Aura, Role uniqueness, token, counter annihilation and repeated SBA fixed point. |
| `rules/turn-structure` | covered; deterministic two-player phase/step shell under restrictions | `turn_structure.rs`, `magic.rs`; rules/environment turn tests | Ordinary priority and stack windows at all boundaries, triggers and cleanup semantics. |
| `rules/zone-incarnation` | covered; selected battlefield→owner graveyard and library-top→hand | `zone_incarnation.rs`; `tests/zone_incarnation.rs` | General zone changes, exile links, dies/ETB events, tokens, cast/resolve destinations and new-object semantics. |
| `rules/basic-land-mana` | specified; subtype-derived intrinsic Basic Land mana requirements | `basic_land_v4.rs`, `basic_land_runtime_v8.rs` execute admitted Basic Land slice; no registry conformance path | Usable narrow mana producer only. Expand costs, restrictions, hybrid choices and generic payment in shared owners. |
| `rules/land-play` | specified; bounded land-play requirements | `basic_land.rs`/V4 and production Basic Land runtime; no registry conformance path | One land-per-turn and timing slice; needs full turn/action/land-history integration. |
| `rules/mana-pool` | specified; six color buckets and two restrictions in state/contract | `mtgml-state` mana payloads, Basic Land V4 consumes narrow subset; no generic payment path | Pool exists; cost comparison, restriction matching, hybrid decisions, payment atomicity and emptying are missing. |

The matrix distinguishes registry status from actual runtime. Current production admission is enforced by `crates/mtgml-card-ir/src/preflight.rs`; `crates/mtgml-rules/src/program_kernel.rs` only admits direct roots `{basic-land-mana, land-play, mana-pool}` for the Basic Land executable profile. The production V8 environment path is `crates/mtgml-environment/src/basic_land_runtime_v8.rs`. The old M3 semantic-ID path is test/historical runtime-testkit, not the current general production path.

## 4. Existing production execution inventory

| Area | Status from Rust implementation | Reusable code / evidence | Missing for locked matchup |
|---|---|---|---|
| Card definition and characteristics | typed contract exists; only Basic Land executable profile; S1-A exact object query and S1-B face/base mapper code exists, latter currently detached/dead-code-allowed | `mtgml-card-ir/src/lib.rs`, `preflight.rs`; `mtgml-rules/src/characteristic_query.rs` | Admit and derive all candidate characteristics; apply counters, continuous effects and attachments; production query path. |
| Object identity, faces, zones | state and transitions exist in bounded forms | `mtgml-state/src/identity.rs`, `zones.rs`, `lifecycle.rs`; `mtgml-rules/src/zone_incarnation.rs` | Generic zone events/incarnation, transform, linked exile and token identity. |
| Turn, priority, combat, draw | bounded implementation exists; not general production executable scope | `magic.rs`, `turn_structure.rs`, `basic_priority.rs`, `combat_damage.rs` and linked tests | Full actions/windows, all steps, choices and reachable combat mechanics. `basic_land::priority_window_after_second_pass` rejects every boundary except Precombat Main → Beginning of Combat. |
| Cast, activate, pay, target, stack | typed V8 payloads and decision purposes exist; semantic producers/consumer absent | `mtgml-state/src/shared_execution.rs`, `execution.rs`, `mtgml-decision/src/v4.rs`, `mtgml-state/src/delta_v3.rs` | Candidate derivation, atomic cost/payment, target declaration/revalidation, stack order and resolve/counter/fizzle. |
| Triggers/replacements/effects | typed records exist; no generic RulesKernel detection/placement/resolution | `shared_execution.rs`, `execution.rs`, `events_v3.rs`; narrow histories in `persisted_v6.rs` | Source-derived trigger detection, APNAP ordering, reflexive choices, replacement ordering and duration semantics. |
| Counters and attachment | state and mutation helpers exist, but no RulesKernel call path | `persisted_v6.rs`, `semantic_mutations.rs` | Counter costs/placement/lore, SBA annihilation, Aura attach legality, Role uniqueness, continuous contribution. |
| SBA | bounded combat SBA only | `state_based_actions.rs` and tests | Full repeat-until-stable set for this matchup: legend, illegal Aura, lethal/zero toughness, counter annihilation, Role uniqueness, token cleanup. |
| Game setup | no production deck-to-game initializer for these lists | `basic_land_runtime_v8.rs` accepts prebuilt `EngineStatePartsV3` and replay manifest | Validated deck list, shuffle/RNG commitment, opening hands, starting player, mulligan/keep and initial observation. |
| Observation/replay/checkpoint/fork | mature typed/persisted contracts and bounded runtime evidence | `mtgml-environment/src/{replay,checkpoint_v8,replay_parity_tests}.rs`, `mtgml-replay/src/v8.rs`, state digests | Prove parity for new action/trigger/effect producers, hidden choices, setup randomness and all new reachable states. Existing Basic Land parity does not generalize. |

G0's V8 state/delta/event/decision/replay family is a contract and persistence substrate. In this audit's source search no general producers were found for `SpellCast`, `StackItemAdded/Removed`, `TriggerDetected/Placed`, generic `CostCommitted`, `TemporaryEffectCreated/Expired`; do not call the typed shapes execution. State `TurnHistory` fields and methods in `semantic_mutations.rs` cover useful data (spells, lost life, red noncombat damage, permanent-to-graveyard, targets, once-use), but outside land-play they are not connected as rules producers for this matchup.

## 5. R1 per-card requirements

All cards below have a validation-only candidate; none except Mountain has executable production profile coverage. “Existing” means bounded substrate only, not that the card is supported. Shared dependencies are detailed in §§7–10. Forge sources and the 96 preserved constructs are listed in `tools/forge-card-script-parser/R1_W1_REPORT.md` and `R1_W1_LOWERING_REPORT.md`.

| Card | Existing semantics | Missing semantics / shared dependencies | Deck-specific requirements | Blockers |
|---|---|---|---|---|
| Fanatical Firebrand | base stats; bounded creature/combat | cast, haste-derived legality, attack/sacrifice ability, target, damage, death/SBA | sacrifice as cost/action; damage trigger and resolution | card unprofiled; no action/stack/target/damage producer |
| Hired Claw | base stats; turn-history fields exist | conditional attack trigger, lost-life condition, pump activation/payment | distinguish opponent lost life this turn from net life; trigger timing | history producer and trigger/activation absent |
| Magebane Lizard | base stats; spell-cast payload shapes | spell-cast trigger and noncreature spell check, damage resolution | count spells cast earlier in turn, including own/opponent | event, history producer, trigger stack and damage absent |
| Emberheart Challenger | base stats; target occurrence history exists | Valiant target trigger, temporary permission/effect, cast-from-exile | “first time targeted this turn”; exile permission normal timing/cost | target history producer, S5/S6a, cast path |
| Razorkin Needlehead | base stats; spell/cast/history contracts | opponent draw trigger and noncombat damage | trigger on each opposing draw | draw event consumer, trigger and damage absent |
| Hearthborn Battler | base stats; bounded combat | second-spell trigger, target opponent, damage | target opponent need not be second-spell caster; attacking trigger | cast history, target, stack and damage absent |
| Ojer Axonil, Deepest Might // Temple of Power | two ordered faces and face state contract; combat damage substrate | red noncombat damage replacement, upkeep transform/return, static damage floor, legend SBA | track controller/source when damage dealt; aggregate red sources; transform face, same permanent identity; Ojer legendary | replacement, source-derived continuous effect, upkeep trigger, full SBA absent |
| Nova Hellkite | base stats; zone/face substrate | Warp alternative permission/cost/exile/timing; cast and resolve | end-step exile and delayed permission/expiry; normal cast alternative | generic cost/cast/stack/zone/timer absent |
| Burst Lightning | printed kicker and cost represented | target spell, optional kicker payment, damage, kicked result | capture whether kicker paid once; target revalidation | S2–S4/S6b absent |
| Lightning Strike | no executable spell semantics; damage data types exist | instant cast, target player/creature, stack, 3 damage and resolution | legal timing, mana/target choices; Dryad/Ojer/Ward interactions | shared casting/payment/targets/stack/damage/replacement/SBA absent |
| Mountain | executable Basic Land profile | only broader turn setup/game support | play once at legal timing; tap for red under current profile | cannot establish complete game; no deck setup |
| Rockface Village | base characteristics only | land play, enters-tapped conditional, activated colored mana, creature-targeted pump, target trigger | T controlled Mouse matters (Emberheart's type); temporal pump | land conditional/ability/castless activation/cost/target/effects absent |

R1 exclusive gap units (two packages): (1) Warp alternate permission and delayed cast boundary (Nova); (2) Ojer transform/face transition and its reachable legendary-state consequences. All other red-only constructs use the shared mechanics above. R1 has four Lightning Strike and Burst witnesses for the same generic instant/kicker path; they are not separate capability requirements.

## 6. W1 per-card requirements

| Card | Existing semantics | Missing semantics / shared dependencies | Deck-specific requirements | Blockers |
|---|---|---|---|---|
| Plains | executable Basic Land profile | setup and full turn integration | white mana production | same setup/full-game blocker as Mountain |
| Ethereal Armor | base characteristics; Aura attachment state | cast/target/attach, static bonus, first-strike granting, characteristic query | counts your Auras attached to any creature, including opponent creature | S1 completion, S2–S4, S2b, attachment and combat extensions |
| Spellbook Vendor | trigger/stack/token typed shapes; attachment state | end-step trigger; optional payment; reflexive targeted trigger; Sorcerer Role creation/attach; +1/+1 counter | two-stage choice/priority timing; Role uniqueness; token not created if illegal | S5, S2, S3/S4, token/object, counters, Aura/SBA |
| Ruin-Lurker Bat | end-step trigger shape; permanent-to-graveyard history field | Descended test; target gain-life trigger; conditional flying counter trigger | permanent cards to own graveyard from any zone this turn; tokens excluded; check at end step | zone/history producer, trigger order/targets, counters and continuous keyword |
| Feather of Flight | Aura base data; attachment state | Aura casting/targeting/attach; +1/+1 and flying until EOT; draw trigger on death | death event, new object and Aura graveyard transition | S3/S4/S5/S6a/S6b, SBA and draw |
| Optimistic Scavenger | enchantment-enter event shape; counter payload | Eerie trigger and +1/+1 counter | triggers once per enchantment entering, even simultaneous; include Aura/Saga/Role | event batching and S5/counter operations |
| Shardmage's Rescue | instant data; target/stack DTOs | cast targeting own creature; ward-like hexproof grant until EOT; cost/payment | target legality and timing; temporary effect | S2–S4, S6a, characteristic/query |
| Sheltered by Ghosts | Aura and exile-link state shapes | cast/attach, ETB trigger, exile target nonland permanent, linked return, ward grant and +1/+1 | source leaves before trigger resolves; target legality; return as new object | S4/S5/S6a/S6b, continuous effect, SBA, zone events |
| Seam Rip | Aura and zone/face payloads | Aura cast/attach; destruction/exile; return Aura from exile if possible with legal attachment (non-target choice) | return attached without targeting; remain exiled if no legal attachment | S3/S4/S5/S6b, attachment and zone incarnation |
| Origin of Spider-Man (alias A Most Helpful Weaver) | face identity + lore counter state; Saga keyword retained | Saga chapter triggers/ordering/sacrifice; Spider token; creature gains legendary Spider Hero; counter placement | two chapter boundaries, legend rule, token object, target choice | S5/S6b, token definitions, legend SBA and characteristic updates |
| Skyward Spider (alias Wonderweave Aerialist) | hybrid cost represented; counter/attachment state | modified query; ward trigger/payment; flying/reach conditional; +1/+1 counter | Aura-controlled-by-creature-controller; counters; Equipment branch unreachable | S1 completion, S2–S5/S6a/S6b; Equipment branch excluded from locked list |
| Abandoned Air Temple | land base data; basic-land query | enters-untapped condition; tap ability producing colorless or conditional white; counter ability | basic lands already controlled; simultaneous entry rulings | land-entry condition, S2, activation, target/counter, land-play history |
| Evershrike's Gift | Aura/counter typed state | cast Blight alternative cost, distribute four -1/-1 counters on one creature; death-return trigger and Aura attach | atomic payment and one chosen creature; counter annihilation; no legal choice if none | S2/S3/S4/S5/S6b, attachment and SBA |
| Dryad Militant | type line; zone transition | graveyard replacement for instant/sorcery cards; hybrid cost/cast | affects both players' instants/sorceries (e.g. Lightning Strike); discard is not replaced | S2 hybrid choice; S6b replacement and destination-aware movement |

W1 exclusive packages (five): Saga chapter/legendary Spider Hero and Spider token; Role creation/attachment/uniqueness; Eerie trigger aggregation; Descended turn history; Aura linked exile/return. This grouping is not one capability per card: counters, Aura attachment, target/stack and event processing are shared owners.

## 7. Shared capability reconciliation

Accepted Shared Foundation dependency DAG: `G0 → S1,S2,S2b,S3,S4,S5`; `S1 → S2,S2b,S3,S6a`; `S2+S3 → S4`; `S4 → S5,S6b`; `S5 → S6a,S6b`; `S6a+S2b → S6b`; `S6b → S7`. S7 is integration and conformance, not runtime rules.

| Node | Baseline status | Evidence and exact gap |
|---|---|---|
| G0 | IMPLEMENTED_FOR_BOUNDED_SCOPE / typed contract family merged | G0j activates V8 state/delta/event/decision/replay contracts. Contracts and validators are not semantic owners. |
| S1 | PARTIALLY_IMPLEMENTED; governance BLOCKED | S1-A exact object/query authority and S1-B face/base mapper exist in `characteristic_query.rs`; base mapper remains detached. Neither covers counter-adjusted values, attachment-derived continuous contributions, nor production admission of R1/W1. Spec and plan still say proposed/no authority, contradictory to merged #255/#256. Reconcile before next S1 work. |
| S2 | DESIGNED_ONLY | Shared cost/payment DTOs exist in `shared_execution.rs` and decision V4; no generic cost derivation, payment candidates/commitment, hybrid-choice payment or atomic cost handling. |
| S2b | DESIGNED_ONLY | No source-derived static layer (Ethereal Armor, Ojer, conditional Skyward); reuse S1 queries and effect/attachment data. |
| S3 | DESIGNED_ONLY | Target-domain DTOs exist; no card-derived target candidate/legality producer or target decision path. |
| S4 | DESIGNED_ONLY | Cast/activation/stack state exists; no general actions, cast pipeline, stack push/pop/resolution or timing legality. |
| S5 | DESIGNED_ONLY | Trigger records/events exist; no source detection, event matching, APNAP ordering, reflexive trigger decision or placement. |
| S6a | DESIGNED_ONLY | Effect/duration data exists; no temporary-effect creation/expiry or layered characteristic consumption. |
| S6b | DESIGNED_ONLY | Replacement/counter/damage/zone typed payloads exist; no ordered replacements, generic counters, damage pipeline, Aura-linked exile, or broad zone-event producer. |
| S7 | NOT_RUN for this task | Existing bounded conformance does not establish R1/W1 closure. Future package needs full cross-owner/replay/privacy evidence. |

Do not start by reimplementing S1-A/B. First reconcile their status and authority; then use the accepted remaining S1 plan (counter-adjusted power/toughness, attachment relations, query closure/conformance) if authorized. Hybrid mana is a concrete contract question for S2: aggregate `ManaCost` and V4 printed-symbol counts have no representation for alternative hybrid payment. Skyward Spider has `WU WU`; Dryad Militant has `GW`. Preserve the single hybrid choice; do not flatten to both colors or pick one.

## 8. Complete turn-structure gap matrix

The existing `turn_structure.rs` has a temporal successor table, not complete rules execution. `magic.rs` and `basic_priority.rs` implement restricted progression. The Basic Land production runtime begins from prebuilt state and its second-pass progression accepts only Precombat Main → Beginning of Combat. These details explain why a visible phase enum is not proof of a playable full turn.

| Phase/step | Existing behavior | Bounded/fail-closed boundary | Needed for R1 × W1 |
|---|---|---|---|
| Beginning: Untap | ordinary untap in M3 shell | no phased-out/skip-untap/card-specific effects | untap permissions and priority boundary integration |
| Upkeep | step representation only | no trigger detection/order/resolution | upkeep triggers (Ojer transformation) and response priority |
| Draw | one library-top→hand draw | no setup/empty-library loss/trigger pipeline | draw event, loss/SBA and Razorkin interaction |
| Precombat main | bounded pass-only window; Basic Land action slice | empty-stack actions; no spell casting | land play, spells/activations, costs, targets, stack and priority |
| Beginning of Combat | temporal step exists | production Basic Land progression can enter from precombat only | trigger/priority window; R1 attack-trigger spell interactions |
| Declare Attackers | <=8 simple bounded choice | no full restrictions/attack triggers | choices, trigger detection/order, priority and state checks |
| Declare Blockers | at most one blocker total | no multiple blockers/full evasion/restrictions | choices and full combat legality |
| Combat Damage | single normal step, simultaneous simple damage | no first/double strike, assignment options, trample/deathtouch/lifelink/replacements | strike-step scheduling, assignments, damage events, Ojer replacement, SBA |
| End of Combat | temporal step only | no complete priority/event behavior | triggers and priority |
| Postcombat main | temporal step only | no generic spell/action window | same action stack as precombat main |
| End Step | boundary exists | no triggers or resolution | Ruin-Lurker, Spellbook Vendor, Emberheart/Fry/Nova timing and triggers; APNAP/order/responses |
| Cleanup | bounded marked-damage reset/handoff | `advance_quiescent_cleanup` does not discard, expire effects, or process triggers; validator rejects unstable states | discard choice, damage removal, EOT expiry, trigger/SBA repeat and extra cleanup if needed |
| Turn transition | deterministic shell | no complete history reset/trigger semantics | upkeep/draw, histories, durations, active-player change, priority |

All boundaries require priority where rules allow it; a spell/ability stack changes progression. State-based actions must run before a player receives priority and repeat to stability, with resulting triggers placed appropriately. Required additions include first/double strike step, damage assignment, multiple blockers, temporary-effect expiry, hand-size discard, cleanup repetition, legal action/target decisions, and turn-history reset. Shared owners S2–S6b must precede full turn composition; no row here should be implemented as a card-specific shortcut.

## 9. Cross-card and generated-object dependencies

| Reachability class | Concrete witness and conclusion |
|---|---|
| REQUIRED_FOR_SHARED_EXECUTION | Game creation for two 60-card lists, seeded shuffle/deal, opening hand and mulligan/keep; ordinary priority/actions; stack; cost/payment; legal targets; event/history; SBA; observation/replay parity. Current runtime starts from already assembled state. |
| REQUIRED_FOR_R1 | Nova Hellkite Warp; Burst Lightning kicker; Emberheart's temporary cast permission; Ojer's transformation and damage replacement; Rockface Village mana/pump; all red creature/spell casting and combat. |
| REQUIRED_FOR_W1 | Aura attachment and static/temporary contributions; Saga chapters; Role/Spider tokens; counter and lore operations; Eerie and Descended; Ward; linked exile/return; hybrid payment. |
| REQUIRED_FOR_CROSS_DECK_INTERACTIONS | R1 burn targets W1 creatures with Ward; Dryad Militant replaces instant/sorcery-to-graveyard moves for both players; Ojer changes R1 red noncombat damage; Sheltered/Seam Rip can remove opponent nonland permanents; R1 can target Emberheart via Rockface; R1 effects interact with W1 Aura attachment and SBAs. |
| NOT_REACHABLE_IN_LOCKED_MATCHUP | Optimistic Scavenger's Room-full-unlock clause: no Room card, effect, or token producer in the two lists. Skyward Spider's “modified” Equipment branch: neither list has Equipment or an Equipment producer. Do not implement those branches for this locked scope absent a new reachable producer. |
| UNKNOWN_REACHABILITY | General engine rules outside the locked two-player ordinary game; anything requiring external format/card producers. Do not claim these are needed or supported. |

Reachable generated objects: Origin of Spider-Man chapter I creates a 2/1 Spider creature token; Spellbook Vendor creates a Sorcerer Role Aura token. Role uniqueness/latest timestamp, token identity/zone cleanup, attach legality and token characteristic definitions are therefore reachable. Ojer is legendary and appears as four copies; Origin chapter II can make multiple creatures legendary Spider Heroes. The CR legend rule (704.5j) is reachable and is marked `LATER_SHARED` in Shared Spec C77, outside the first S1–S6b cut; schedule it before claiming full R1/W1 closure. M3 combat SBA rejects non-card/token objects and does not enforce legend or Aura legality.

Specific recursion/interaction checks:

- **Ojer Axonil:** face transform upkeep trigger; preserve permanent identity across face change; red noncombat damage floor replacement applies when damage is dealt and can aggregate damage from sources; its legendary status makes duplicates relevant. No transformation/replacement/legend implementation exists.
- **Nova Hellkite Warp:** exile with delayed permission to cast later at alternate cost/timing. Needs zone incarnation, duration/history and ordinary cast lifecycle; do not treat as a separate card executor.
- **Burst Lightning:** optional kicker paid once as part of total cost and “kicked” captured for resolution. Generic cost/payment/cast path must preserve the choice.
- **Emberheart Challenger:** Valiant is based on a creature becoming target of a spell/ability for first time each turn, plus temporary cast permission. History fields exist; producers and expiry do not.
- **Ward:** Skyward Spider and Sheltered by Ghosts require a generic target-trigger/payment/counter stack path for opposing targets.
- **Auras:** Ward-like grants, targeted Aura spells, attach/illegal attach, static contributions, death/zone changes and linked exile/return must use general object semantics. Sheltered's linked cards return as new objects; Seam Rip's Aura return chooses a legal attachment without targeting and stays exiled if none.
- **Origin Saga:** lore counters, chapter trigger order, chapter I token, chapter II legendary type changes, final chapter and sacrifice/zone change; subsequent legend SBA is reachable.
- **Dryad Militant:** replacement affects instant/sorcery cards going to graveyard from anywhere for either player; discard is not replaced. Lightning Strike and Burst are concrete interaction witnesses.
- **Other cards:** Ruin-Lurker counts permanent cards put into own graveyard from any zone this turn (not tokens); Vendor's reflexive trigger chooses its target after payment; Scavenger triggers once per enchantment entering in a simultaneous batch; Evershrike's Blight payment is atomic and places all counters on one creature.

Oracle/rulings authority used for these readings was pinned as listed in §1; 59 rulings for selected Oracle IDs were checked. Forge A/T/R/S/K/SVar are only source descriptions. They are not the rules authority and were not executed. The Forge Oracle-property textual deviations noted in §1 are reminder/name wording; no operative textual mismatch was found in that property comparison, but that does not settle Forge-script mapping accuracy.

## 10. Prioritized implementation dependency graph

Do not authorize work solely from this research document. Resolve the S1 governance contradiction first. Work packages below reuse the accepted owner sequence and identify the smallest sensible acceptance scope; they are not card-by-card PRs.

| Order/package | Exact semantic scope | Reuse / missing work | Dependencies and affected contracts | Card witnesses / acceptance evidence |
|---|---|---|---|---|
| 0. S1 authority reconciliation | Mark S1 spec/plan status consistent with merged A/B or explicitly resolve the discrepancy; no rule changes | Reuse PR #255/#256 and current S1 documents; determine authority before S1-C | Docs/ADR/status only; no production contract change | Show accepted owner/status and authorized remaining plan; blocker resolved before S1 extension. |
| 1. S1 completion | Counter-adjusted characteristics, attachment queries, characteristic closure/conformance | Reuse `characteristic_query.rs`, Card IR face/base mapper, counter/attachment state; add connected production derivation and tests | S1 contract and current effect/query payloads | Ethereal Armor, Ojer floor, Skyward modified, Saga type changes; exact object/face and canonical/replay stability. |
| 2. S2 cost/payment + S2b static contribution | Cost candidates, generic/colored/colorless/hybrid alternatives, atomic payment; source-derived static characteristics | Reuse mana pool and V4 decisions; specify hybrid choice before implementation; connect S1 queries | Mana/payment and decision payload compatibility; no flattening `GW`/`WU` | All nonland spells, Rockface/Village, Armor/Ojer; deterministic legal payments, restrictions, nonmutation on failure. |
| 3. S3 targets | Reusable target domains, legality, choices and revalidation | Reuse target DTOs/history; create candidates from current state and action source | Decision V4 target choice; object/zone incarnation | Lightning Strike, Ward, Auras, Vendor, Saga; legality at declaration/resolution and replay parity. |
| 4. S4 casting/activation/stack | Timing, cast/activation costs, stack object lifecycle, resolution/result destinations | Reuse state `ExecutionStateV4`, delta/event/replay shapes; add RulesKernel producers/consumer | S2 + S3; priority action closure | Mountain/Plains remain baseline; Lightning Strike/Burst/Rockface actions; cast/stack events and deterministic resolve. |
| 5. S5 trigger detection and ordering | Generic event-to-trigger derivation, APNAP ordering, reflexive decisions, stack placement | Reuse event records/history and S4 stack | S4 and event producer closure | Magebane, Ward, Eerie, Saga, Vendor, Ruin-Lurker; batch/simultaneous event and ordering conformance. |
| 6. S6a temporary effects | Create/read/expire EOT and delayed permission effects | Reuse effect state and S1/S2b characteristic query | S1, S2b, S4, S5 | Feather, Shardmage, Sheltered, Emberheart, Rockface, Nova; exact expiry across cleanup/replay. |
| 7. S6b counters/replacements/damage/zones/Auras | Atomic counters/annihilation, ordered replacements, noncombat damage, linked exile/return, general zone events | Reuse `semantic_mutations.rs`, `zones.rs`, event/delta payloads, bounded damage/SBA; wire into kernel | S4/S5/S6a; S1/S2b queries | Ojer, Dryad, Evershrike, Sheltered, Seam Rip, Origin; replacement ordering, new-object return, event/history evidence. |
| 8. SBA and reachable closure extension | Repeat-until-stable SBAs: legend, illegal Aura, zero/lethal, counter annihilation, Role uniqueness, tokens | Reuse bounded `state_based_actions.rs`; extend state validation and generated-object handling | S1, S5, S6b | Ojer duplicates, Origin Spider Heroes, Role uniqueness, Aura detach; rule-order/fixed-point cases. This closes C77 deferred item. |
| 9. Game setup | Locked-list validation, deterministic shuffle/deal, opening choices/mulligan, initial player/observation | No current producer; reuse replay RNG/provenance/privacy contracts | Deck/format contract and decision/observation/replay | Exact Issue #222 manifests/aliases; seed replay, hidden-information noninterference, mulligan evidence. |
| 10. Ordinary turn composition and S7 | Compose all phases/steps, priority/stack/SBA/trigger windows, combat strike steps, cleanup and repeated cleanup | Reuse M3 temporal shell and V8 runtime/replay integration; broaden boundaries instead of duplicating turn owner | Packages 1–9 as applicable; conformance/replay/privacy | Full turn scenarios spanning both lists; first/double strike, multiple blockers, discard, expiry, turn history and checkpoint/fork/replay parity. |

Packages 1–8 follow Shared owners; package 9 is an independent prerequisite to complete games and can be designed in parallel only if it does not invent conflicting decision/replay contracts. Package 10 is integration, not an alternate rules engine. Exact order among S2/S2b/S3 follows the accepted DAG; do not start S4 without cost and target contracts. S7 evidence must include cross-card and multi-turn interactions, not just successful individual card actions.

## 11. Open questions and blockers

1. **S1 authority/status contradiction:** merged S1-A/B implementation versus still-proposed spec/plan (“Implementation authorized: NO”). Reconcile before additional S1 implementation or describing S1 as accepted/covered.
2. **Hybrid payment contract:** current shared `ManaCost` / V4 symbol counts do not encode alternative hybrid payment. Decide a typed representation without changing printed symbols or silently choosing a mode.
3. **Setup semantics:** no production deck initializer exists. Decide/identify the existing accepted owner for randomness, mulligan and starting player before game-level acceptance.
4. **Capability registry granularity:** three specified entries overlap an executing Basic Land profile but registry lifecycle is not promoted. Keep current registry unchanged; future registry updates require their own evidence/lifecycle process.
5. **Full game scope:** ordinary two-player constructed game is assumed from #222, but format/setup contract and exact sideboard policy are not locked in the code. No sideboard cards were added to this audit.
6. **External authorities:** Oracle and rulings were used for selected rules; the accepted CR snapshot is the rule authority. Forge script execution details are evidence only. No Forge or Oracle runtime dependency is proposed.

No implementation blocker prevents future work from being scoped. The first governance blocker is the S1 authority inconsistency; the hybrid-cost and game-setup owners need explicit contract decisions before their dependent execution work.

## 12. Evidence and verification status

Implementation claims were checked against Rust modules named above and existing tests; the registry and candidate artifacts were read from this baseline. Accepted sources inspected include the M3 Foundation V2 semantic spec, accepted M4 Shared Execution Foundation spec/plan, accepted G0 spec/plan, and S1 characteristic/object-query spec/plan. S1 status contradiction is stated rather than resolved in this audit. Issue #237's old research inventory was not used as current semantic evidence.

Existing relevant evidence includes `crates/mtgml-rules/src/tests/{basic_priority,turn_structure,state_based_actions,zone_incarnation}.rs`, `tests/combat_damage` coverage in `batch_e.rs`, environment `tests/{magic_rules_production,turn_structure,replay_parity_tests}.rs`, state validation/mutation tests, and Basic Land V8 runtime tests. These tests evidence their bounded scopes only.

This audit introduced no code, schema, registry or test changes. Per research-only instruction, no tests or repository checks were run; they are `NOT_RUN`, not passed. No full R1/W1 conformance, executable profile validation, game setup, or replay parity exists on this baseline.
