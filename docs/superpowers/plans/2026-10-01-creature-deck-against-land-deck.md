# Creature Deck Against a Land Deck Implementation Plan

**Status:** DRAFT for the owner's review, 2026-10-01. It implements steps 1 and 2 of
the approved spec. Steps 3a and 3b (blocks, damage division, symmetric smoke
games) get their own plans.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** a deck of lands and vanilla creatures (Savannah Lions, Gray Ogre, Hill
Giant) casts its creatures, attacks with them and kills a land-only opponent,
through the production endpoints and in random smoke games.

**Architecture:**
- A second card profile, `vanilla-creature@1.0.0`, is admitted from pinned
  Oracle records with expected characteristics.
- Casting goes through a one-item stack: the card moves hand → stack, the cost
  is paid from the mana pool, and both players pass to resolve it onto the
  battlefield.
- A `ManaPayment` decision is asked only when payments leave different pools.
- `controlled_since_turn` in the card-rules state decides which creatures can
  attack. Unblocked attackers cause life loss, and CR 704.5a ends the game.
- No card-name dispatch: every rule reads the verified catalog's base
  characteristics through the S1 query.

**Tech Stack:** Rust workspace (`mtgml-card-ir`, `-persistence`, `-state`,
`-rules`, `-observation`, `-environment`, `-wire`), Python client mirror, JSON
schemas.

**Spec:** `docs/superpowers/specs/2026-10-01-vanilla-creatures-design.md`
(revision 2, approved 2026-10-01). Its §7 step 2 is this plan's target. Its §3
rules are cited by number below.

## Global Constraints

- AGENTS.md is binding:
  - correctness → determinism → information safety → decision completeness;
  - every player choice is an explicit decision;
  - unsupported rules fail closed with a precise error;
  - formats change in place (no `FooV9` next to `FooV8`; delete replaced code
    in the same change);
  - tests go through the production runtime.
- **Every commit** keeps green: `cargo fmt --all -- --check`, clippy
  `-D warnings`, `cargo test --workspace --locked`, the release `random_smoke`,
  the Python suite, ruff, mypy and the fast-gate scripts. Commit before running
  the gate scripts.
- **A commit that changes the smoke pins** re-pins them. Its message must show
  that only what players see or the digest changed: a scratch fingerprint over
  actor, response bytes and checkpoint digest must be identical before and
  after for the land-only games, or, when the digest itself changes, over
  actor and response bytes only.
- **Rules text:** read it from the pinned TXT
  `C:\Dev\src\Manafold\.rules\MagicCompRules 20260925.txt` (SHA-256
  `8d860e45…`). It is git-ignored and must never be committed.
- **Oracle archive:** `C:\Dev\src\Manafold\.oracle\oracle-cards-20260925210158.jsonl.gz`
  (SHA-256 `c607300f…`). It is git-ignored and must never be committed.
- Python runs with `C:\Dev\src\Manafold\.venv`. `PYTHONPATH` uses `;` on
  Windows.
- **Definition ids:** Mountain 1, Plains 2, Savannah Lions 3, Gray Ogre 4,
  Hill Giant 5.
- **Mana buckets** `[u32; 12]`: indexes 0–5 are unrestricted W, U, B, R, G, C;
  indexes 6–11 are creature-spell-only W, U, B, R, G, C.

## Review Focus

1. **Responses:** while a spell is on the stack, neither player may cast or
   play a land. Only passing and mana abilities are offered. Tested in Task 5:
   `the_opponent_can_only_pass_or_make_mana_while_a_spell_is_on_the_stack`.
2. **Land-only admission:** a creature definition under the land-only
   admission must be refused precisely. Tested in Task 3:
   `a_creature_under_the_land_only_admission_is_refused`.
3. **Restore mid-cast:** a checkpoint taken while a ManaPayment is pending must
   restore and continue identically. Tested in Task 6:
   `a_restored_payment_checkpoint_continues_identically`.
4. **Overkill:** damage that takes life below 0 still ends the game, and the
   observation shows the negative life. Tested in Task 7:
   `overkill_damage_shows_negative_life_and_ends_the_game`.
5. **A game ending before turn 31:** the thirty-turn smoke test must accept a
   terminal status. Tested in Task 9: the asymmetric smoke test asserts
   `EpisodeStatus::Terminal` and checks replay equality for a game that ends at
   0 life.

---

### Task 1: The vanilla-creature profile in the card IR

**Files:**
- Modify: `crates/mtgml-card-ir/src/lib.rs`: profile constants,
  `CardSemanticBindingV1`, validation (~:369–:433), codec (~:897–:921,
  ~:1049–:1065).
- Modify: `crates/mtgml-persistence/src/content_contract_digest.rs:50-66`: the
  profiled-identity gate.
- Modify: `python/src/mtgml/content_contract_v1.py:151-175`, plus
  `crates/mtgml-card-ir/tests/fixtures/content_contract_manifest_parity.v1.json`.
- Modify, because `body.subtype` accesses must match on the enum:
  - `crates/mtgml-rules/src/basic_land.rs` (~:537);
  - `crates/mtgml-environment/src/basic_land_runtime_v8.rs` (:568, :578,
    :1108, :1437);
  - `crates/mtgml-environment/tests/common/mod.rs:103`;
  - `crates/mtgml-environment/tests/current_successor_api.rs:286`;
  - `crates/mtgml-environment/src/tests/magic_basic_land_observation.rs:66-75`;
  - `crates/mtgml-card-ir/tests/profiled_content_prerequisite.rs:65-77`.
- Test: `crates/mtgml-card-ir/tests/vanilla_creature_profile.rs` (new);
  `python/tests/test_content_contract_v1.py` (add cases).

**Interfaces:**
- Produces:
  - `pub const VANILLA_CREATURE_PROFILE_ID_V1: &str = "vanilla-creature@1.0.0";`
  - `pub const VANILLA_CREATURE_PROFILE_BODY_V1: &str = "vanilla-creature-profile.v1";`
  - `pub enum CardProfileBodyV1 { BasicLand(BasicLandProfileV1), VanillaCreature }`
    (Copy, Ord). `CardSemanticBindingV1::ProfiledV1 { profile_id, body: CardProfileBodyV1 }`.
  - Wire value of the vanilla body: `["vanilla-creature-profile.v1", null]`.
    The basic-land body bytes are unchanged.

- [ ] **Step 1: Write the failing tests** in `vanilla_creature_profile.rs`:
  - `a_vanilla_creature_definition_round_trips`: Savannah Lions (one face
    `FaceKey(0)`, name "Savannah Lions", cost `[White]`, type line
    `[]/["Creature"]/["Cat"]`, P/T `(2, 1)`, no abilities, no color indicator)
    encodes and decodes to equal bytes.
  - `a_vanilla_creature_with_rules_or_bad_shape_is_rejected`: each of the
    following fails validation:
    - toughness 0;
    - power −1;
    - an ability identity;
    - card types `["Creature","Artifact"]`;
    - a supertype;
    - no mana cost;
    - two faces;
    - loyalty `Some`.
  - `the_basic_land_bytes_are_unchanged`: the committed
    `cards/definitions/basic-land-v1/content-contract.v1.cbor` decodes and
    re-encodes to the same bytes and the same content id `80d26c18…`.
  - Python: the same Lions manifest is accepted, and a body label
    `"vanilla-creature-profile.v2"` is rejected (`decode.invalid_json`).
- [ ] **Step 2: Run** `cargo test -p mtgml-card-ir --test vanilla_creature_profile --locked`.
  Expected: FAIL to compile (the constants and enum are missing).
- [ ] **Step 3: Implement**
  - Add the enum.
  - `validate_vanilla_creature_profile(definition) -> Result<(), ContentValidationErrorV1>`:
    - exactly one face `FaceKey(0)`;
    - no supertypes;
    - card types exactly `["Creature"]`, subtypes non-empty;
    - cost `Some` and non-empty, no hybrid symbols;
    - P/T `Some((p, t))` with `p ≥ 0` and `t ≥ 1`;
    - loyalty and defense `None`;
    - empty color indicator;
    - no ability identities.
  - Extend the codec and the persistence identity gate. A missing gate makes
    content ids fail with `UnknownVariant`.
  - Extend the Python mirror and the parity fixture with one vanilla case.
- [ ] **Step 4: Run** the card-ir tests, `cargo test --workspace --locked`, and
  the Python suite. Expected: PASS, and the basic-land KAT is unchanged.
- [ ] **Step 5: Commit** `feat: a vanilla-creature card profile`.

### Task 2: Pinned witness records and the combined catalog

**Files:**
- Modify: `crates/mtgml-card-ir/src/preflight.rs`:
  - `ORACLE_SNAPSHOT` → per-record snapshot;
  - `BASIC_LAND_SOURCE_RECORDS` → `PINNED_ORACLE_RECORDS`;
  - `validate_pinned_basic_land_profile` → `validate_pinned_profiles`;
  - `derived_requirement_roots` (:253).
- Create: `cards/definitions/basic-land-and-vanilla-creature-v1/` with
  `content-contract.v1.cbor`, `provenance.v1.cbor` and `README.md` (pins and
  the record table, as in the spec header).
- Create: `persistence/golden/content-contract-basic-land-and-vanilla-creature-v1-kat.v1.json`,
  registered wherever `content-contract-basic-land-v1-kat.v1.json` is (grep the
  file name).
- Modify: `cards/capabilities/registry.json`:
  - new `rules/cast-creature-spell`, `rules/stack-resolution` and
    `rules/summoning-sickness` @0.1.0, lifecycle `specified`;
  - in-place summaries for `rules/declare-blockers`, `rules/combat-damage` and
    `rules/damage-and-life`, describing the spec's multi-blocker kernel.
- Create the capability docs with `scripts/scaffold_capability.py`, then
  regenerate with `scripts/generate_card_ir_capability_projection.py`.
- Modify: `scripts/extract_oracle_records.py`: the vanilla checks (`layout`
  normal, no `card_faces`, empty `oracle_text` and `keywords`, integer P/T,
  toughness ≥ 1) and `--oracle-id` lookup. Test:
  `python/tests/test_oracle_records.py`.
- Test: `crates/mtgml-card-ir/tests/profiled_content_admission.rs` (add tests);
  `crates/mtgml-card-ir/tests/combined_catalog.rs` (new).

**Interfaces:**
- Produces:
  - `pub fn admit_executable_profile_v1(..)` with an unchanged signature; it now
    admits both profiles.
  - Derived roots of `vanilla-creature@1.0.0`: `rules/cast-creature-spell`,
    `rules/stack-resolution`, `rules/summoning-sickness`,
    `rules/combat-damage`, `rules/damage-and-life` and
    `rules/state-based-actions-combat` (all @0.1.0), plus registry
    dependencies.
  - **Ruling:** these are profile roots, not `MAGIC_GAME_RULE_ROOTS`. A
    land-only admission keeps its closure and its rules contract id, so
    land-only games keep their bytes.

- [ ] **Step 1: Write the failing tests:**
  - `combined_catalog_bytes_are_canonical`: the committed CBOR decodes,
    re-encodes byte-identically, and its content id equals the KAT's.
  - `the_combined_catalog_is_admitted_with_creature_roots`: admission
    succeeds, and the resolved closure contains the six roots above.
  - `a_creature_record_with_other_characteristics_is_refused`: the Lions
    provenance with a 9/9 body gives `PinnedSourceProvenanceMismatch`.
  - `an_unpinned_oracle_record_is_refused`: an unknown `oracle_id` gives
    `PinnedSourceProvenanceMismatch`.
  - `the_land_only_admission_is_unchanged`: the basic-land admission has the
    same closure and rules contract id as before (pin the id from `master`).
- [ ] **Step 2: Run** `cargo test -p mtgml-card-ir --locked`. Expected: FAIL
  (missing files, unknown records).
- [ ] **Step 3: Implement** `PinnedOracleRecordV1 { snapshot: &'static str, oracle_id: &'static str, record_sha256: &'static str, expected: PinnedCharacteristicsV1 }`
  with
  `enum PinnedCharacteristicsV1 { BasicLand(BasicLandSubtypeV1), VanillaCreature { name, mana_cost: &'static [PrintedManaSymbolV1], subtypes: &'static [&'static str], power: i32, toughness: i32 } }`.
  - The five records:
    - Mountain and Plains as today;
    - the Lions, Ogre and Giant rows from the spec header, with snapshot
      `oracle-cards-20260925210158`.
  - Every definition must match a record exactly: snapshot, codec, oracle_id,
    digest, profile and characteristics.
  - Duplicates are refused. Drop the "exactly two definitions" rule.
  - Generate the CBOR once with an `#[ignore]` test
    `writes_the_combined_catalog` that encodes the five definitions with the
    crate's manifest encoder (the one the decoder's re-encode check calls).
    Commit the bytes.
- [ ] **Step 4: Run** the card-ir tests, the workspace tests,
  `scripts/validate_maintainer_artifacts.py`,
  `scripts/generate_card_ir_capability_projection.py --check` and
  `scripts/verify_repository.py`. Expected: PASS, and the land smoke pins are
  unchanged.
- [ ] **Step 5: Commit** `feat: admit pinned vanilla creatures in a combined catalog`.

### Task 3: Creature definitions reach a real game

**Files:**
- Modify: `crates/mtgml-rules/src/characteristic_query.rs` (:143, :256):
  `is_admitted_basic_land_profile` → `is_admitted_profile` (both profiles).
- Modify: `crates/mtgml-rules/src/game_start.rs:85-89`: admit
  `VANILLA_CREATURE_PROFILE_ID_V1`.
- Modify: `crates/mtgml-environment/tests/common/mod.rs`:
  - `creature_game_admission()`;
  - `creature_definitions() -> [CardDefinitionId; 3]`;
  - `creature_deck_game(decks, seed)`;
  - `controller_for` takes the catalog bytes used by `replay_manifest`.
- Test: `crates/mtgml-environment/tests/creature_game.rs` (new).

**Interfaces:**
- Produces: the test helpers above. `creature_definitions()` returns
  `[CardDefinitionId(3), CardDefinitionId(4), CardDefinitionId(5)]`.

- [ ] **Step 1: Write the failing tests:**
  - `a_deck_with_creatures_starts_a_game`: a 20-land + 7-creature deck against
    27 lands goes through start, keep and keep. The creatures sit in hand or
    library, and turn 1 runs to cleanup with land plays only.
  - `a_creature_under_the_land_only_admission_is_refused`: `start_game` with
    the land-only admission and a deck containing id 3 gives
    `GameStartError::UnknownDefinition`.
- [ ] **Step 2: Run** `cargo test -p mtgml-environment --test creature_game --locked`.
  Expected: FAIL (`UnsupportedDefinition`).
- [ ] **Step 3: Implement** the two rule-crate changes and the helpers. No
  candidate for a creature card exists yet: `derive_basic_land_candidates`
  skips non-land cards (~:769).
- [ ] **Step 4: Run** the workspace tests. Expected: PASS.
- [ ] **Step 5: Commit** `feat: games start from decks with vanilla creatures`.

### Task 4: `controlled_since_turn` as a card-rules source fact

**Files:**
- Modify: `crates/mtgml-state/src/card_rules.rs`: the `PermanentState` family
  and `canonical_value` (append an 8th element).
- Modify: `crates/mtgml-state/src/semantic_mutations.rs`: `enter` and
  `prune_departed_objects`.
- Modify: `crates/mtgml-state/src/engine.rs`: `validate_card_rules`.
- Modify: `crates/mtgml-state/src/delta.rs`: coverage.
- Modify: `crates/mtgml-state/src/lib.rs`: exports.
- Modify: `crates/mtgml-state/src/construction.rs`.
- Modify: `crates/mtgml-rules/src/basic_land.rs`: play land (~:283-286,
  ~:406).
- Modify: `crates/mtgml-rules/src/turn_progression.rs`: `move_card` (prune on
  battlefield departure).
- Modify: `docs/STATE_HASHING.md` (:53, :769-788).
- Re-pin:
  - `crates/mtgml-state/tests/g0e_digest.rs` (~:224);
  - `crates/mtgml-replay/src/v8.rs:434`;
  - `crates/mtgml-environment/tests/random_smoke.rs`.
- Test: `crates/mtgml-state/src/tests/digest.rs`
  (`v7_digest_binds_each_card_rules_family`), plus a new
  `crates/mtgml-state/src/tests/permanents.rs`.

**Interfaces:**
- Produces:
  - `pub struct PermanentState { pub controlled_since_turn: u64 }`
  - `pub struct PermanentsState { pub permanents: BTreeMap<GameObjectId, PermanentState> }`
  - The card-rules field is `permanents: PermanentsState`.
  - `PermanentsState::enter(&mut self, object: GameObjectId, turn: u64) -> Result<(), StateFamilyMutationError>`
    fails on a duplicate.
  - `PermanentsState::prune_departed_objects(&mut self, battlefield: &BTreeSet<GameObjectId>)`.
  - Digest element 8 is a sorted `[[object, controlled_since_turn], …]` list.
- **Ruling:** `marked_damage` (spec §4) arrives with step 3a, its first writer.
  An always-zero field would be untested. Cost: one more in-place digest
  change in 3a.

- [ ] **Step 1: Write the failing tests:**
  - `a_land_played_on_turn_one_is_controlled_since_turn_one`: real game flow
    through `install_basic_land_request` and a PlayLand answer.
  - Validation rejects:
    - a battlefield object without an entry;
    - an entry for a library object;
    - `controlled_since_turn > core.turn_number`.

    All of these apply only when content authority is present, mirroring
    faces.
  - Delta rejects:
    - an entry added without the object entering the battlefield;
    - an entry whose turn differs from `after.core.turn_number`;
    - an entry changed while the object stays.
  - The digest mutation test gains a `permanents` case.
- [ ] **Step 2: Run** `cargo test -p mtgml-state --locked`. Expected: FAIL to
  compile.
- [ ] **Step 3: Implement** as specified.
- [ ] **Step 4: Run** the workspace tests and the release smoke.
  - Expected: the digest pins and both smoke pins FAIL with new values.
  - Re-pin them. Prove with the scratch fingerprint over actor and response
    bytes that the land games' decisions are unchanged. Ledger the values.
- [ ] **Step 5: Commit** `feat: permanents record since when their controller controls them`.
  The message carries the pin evidence.

### Task 5: Cast a creature with a single payment, and resolve it

**Files:**
- Create: `crates/mtgml-rules/src/casting.rs` (`pub(crate)` module).
- Modify: `crates/mtgml-rules/src/zone_incarnation.rs`:
  `SelectedZoneTransitionKind::HandToStack` and `StackToBattlefield { controller: PlayerId }`.
- Modify: `crates/mtgml-rules/src/basic_land.rs`:
  - `derive_basic_land_candidates` (CastSpell);
  - `selected_basic_land_action` (:1011, :1064);
  - `candidate_state` (:645-664, :897-906);
  - visibility `ActingPlayerOnly` (:927, :984).
- Modify: `crates/mtgml-rules/src/turn_progression.rs`:
  - `execute_magic_response`;
  - `validate_slice` (:245-282);
  - `progress` (:334: a second pass with a non-empty stack resolves);
  - `finish` (`StackOrderChanged` op).
- Modify: `crates/mtgml-rules/src/events.rs`: `is_projectable_public_source_event`
  stays as it is for SpellCast and CostCommitted (trusted).
- Modify: `crates/mtgml-environment/src/successor_projection.rs`: the existing
  StackItemAdded and StackItemRemoved arms must accept a vanilla spell.
- Modify: `cards/capabilities/registry.json` and the capability docs:
  `rules/cast-creature-spell` and `rules/stack-resolution` become `covered`.
- Re-pin the smoke games: the priority request's visibility changes.
- Test: `crates/mtgml-environment/tests/creature_game.rs`; unit tests in
  `casting.rs`.

**Interfaces:**
- Produces:
  - `pub(crate) fn mana_cost_of(face: &FaceDefinitionV1) -> Result<ManaCost, CastError>`.
    Hybrid fails closed.
  - `pub(crate) fn payment_options(pool: &ManaPoolV1, cost: &ManaCost) -> Vec<[u32; 12]>`:
    every spend vector that pays the cost exactly, grouped by remaining pool,
    with the lexicographically smallest vector kept per group, sorted
    ascending. It is empty if the pool cannot pay.
  - `pub(crate) fn cast_spell(next: &mut EngineState, caster: PlayerId, card: GameObjectId, spent: [u32; 12], facts: &mut Facts) -> Result<StackObjectId, Error>`.
  - `pub(crate) fn resolve_top(next: &mut EngineState, facts: &mut Facts) -> Result<(), Error>`.
- Consumes: Task 4's `PermanentsState::enter`.

- [ ] **Step 1: Write the failing tests** in `creature_game.rs` (all through
  the endpoints):
  - `a_creature_is_cast_and_resolves_onto_the_battlefield`:
    - Turn 1, the player with Plains: play Plains, tap it.
    - A `CastSpell` candidate for the Lions appears; choose it.
    - The stack shows the spell for both players, and the caster has priority
      (CR 117.3c).
    - Both pass. The Lions is on the battlefield under the caster with
      `controlled_since_turn` equal to the turn number, the active player has
      priority (117.3b), and the step has not changed.
  - `casting_is_offered_only_at_sorcery_speed_with_an_exact_payment`: no
    `CastSpell` is offered when:
    - the pool cannot pay;
    - it is not a main phase;
    - the player is not active;
    - the stack is non-empty.
  - `the_opponent_can_only_pass_or_make_mana_while_a_spell_is_on_the_stack`.
  - `the_opponent_learns_the_card_only_when_it_is_cast`: paired-state
    noninterference. Two games differing only in the caster's other hand card
    give byte-identical opponent information states until the cast, and after
    it.
  - Unit tests:
    - `payment_options` for Gray Ogre `{2}{R}` and pool R=3, W=1 gives exactly
      two options (one leaves W=1, one leaves R=1);
    - with pool R=1 it gives none.
- [ ] **Step 2: Run** `cargo test -p mtgml-environment --test creature_game --locked`.
  Expected: FAIL (no CastSpell candidate).
- [ ] **Step 3: Implement.**
  - **Candidates:** CastSpell for each hand card with the vanilla profile.
    Conditions:
    - the actor is active and holds priority;
    - main phase, empty stack;
    - capability `rules/cast-creature-spell`;
    - **exactly one** payment option. With two or more, Task 6 adds the
      decision. Until then those cards are not offered, which is ledgered.
  - **Cast,** atomic in one transition:
    1. `HandToStack` zone transition (perspectives as on the discard path);
    2. new `StackRecord` with `Spell` payload;
    3. `StackItemAdded`;
    4. `SpellCast { is_creature_spell: true, .. }`;
    5. `CostCommitted { action: Cast, spent_buckets }`;
    6. spend the pool through the existing ManaPoolChanged event's spending
       cause;
    7. `turn_history.record_spell_cast(caster, false)`;
    8. `StackOrderChanged` op.

    Then priority goes to the caster with passes reset.
  - **Resolve,** on the second consecutive pass with a non-empty stack:
    1. `StackItemRemoved { result: Resolved }`;
    2. `StackToBattlefield { controller }`;
    3. `permanents.enter(new_object, turn_number)`;
    4. `StackOrderChanged`.

    Then `open_priority` without a position change (pools keep their mana).
  - `validate_slice` and `candidate_state` accept one `Spell` stack item whose
    card has the vanilla profile, including terminal and truncated
    checkpoints.
- [ ] **Step 4: Run** the workspace tests and the release smoke.
  - Expected: PASS after re-pinning. The land games' scratch fingerprint over
    actor, response and checkpoint digest is unchanged, because only the
    request visibility changed.
- [ ] **Step 5: Commit** `feat: cast a vanilla creature through a one-item stack`.

### Task 6: ManaPayment when payments leave different pools

**Files:**
- Modify: `crates/mtgml-rules/src/casting.rs`: `begin_cast` and
  `complete_cast`.
- Modify: `crates/mtgml-rules/src/basic_land.rs`: offer CastSpell when there
  are two or more options.
- Modify: `crates/mtgml-rules/src/turn_progression.rs`:
  `execute_magic_response`, `validate_magic_pending_request` and
  `validate_slice` (one Cast continuation with a pending ManaPayment).
- Modify: `crates/mtgml-state/src/delta.rs:536-582,1179-1212`: SpellCast
  timing.
- Modify: `crates/mtgml-state/src/engine.rs`:
  - Cast validation (~:760-785): the `spell_object` is on the stack with a
    matching Spell record;
  - PayingMana requires `AwaitingFinalAllocation` with **no** source
    activations.
- Modify: `crates/mtgml-state/src/shared_execution.rs:1-5`: drop the stale
  "not connected to any writer" doc comment.
- Modify: `crates/mtgml-rules/src/events.rs`: SpellCast projection (~:1442) and
  `validate_cost_commit_projection` (~:1860, :1908, so that a missing staged
  cast fails instead of being skipped).
- Test: `crates/mtgml-environment/tests/creature_game.rs`; state and events
  unit tests.

**Interfaces:**
- Produces:
  - `pub(crate) fn begin_cast(next, caster, card, options: Vec<[u32; 12]>, facts) -> Result<(), Error>`:
    hand → stack, `StackItemAdded`, the Cast continuation, and a
    `ManaPayment` request (`ChooseOne`, `ActingPlayerOnly`) with one
    `SelectManaPayment { spent_buckets }` per option in `assign_dense` order.
  - `pub(crate) fn complete_cast(next, spent: [u32; 12], facts) -> Result<(), Error>`:
    `SpellCast`, `CostCommitted`, the spending and `record_spell_cast`, then
    the continuation is removed (`ContinuationChanged`) and the caster gets
    priority.
- **Ruling (spec §3, CR 601.2a/601.2i):** the card is on the stack while the
  player pays, and `SpellCast` fires only when payment completes. The delta
  rule changes in place:
  - a new stack record needs `StackItemCreated`;
  - `SpellCast` and the `spells_cast_total` increase belong to the transition
    that creates the record **or** the one that removes that card's Cast
    continuation.

- [ ] **Step 1: Write the failing tests:**
  - `gray_ogre_with_three_mountains_and_a_plains_asks_how_to_pay`:
    - Tap three Mountains and a Plains, then cast the Ogre.
    - Expect a `ManaPayment` request with exactly two candidates.
    - Choosing the one that keeps W leaves pool W=1; the other leaves R=1.
    - SpellCast appears only in the payment step.
  - `a_restored_payment_checkpoint_continues_identically`.
  - `a_rejected_payment_answer_changes_nothing`.
  - State: a Cast continuation with a source activation in PayingMana is
    rejected.
  - Delta: a stack record created without `StackItemCreated` is rejected; so is
    a `SpellCast` in a transition that neither creates the record nor ends its
    continuation.
- [ ] **Step 2: Run** them. Expected: FAIL (no request; the delta rejects a
  record without SpellCast).
- [ ] **Step 3: Implement** as specified.
- [ ] **Step 4: Run** the workspace tests and the release smoke. Expected:
  PASS, and the land pins are unchanged.
- [ ] **Step 5: Commit** `feat: choose how to pay when payments differ`.

### Task 7: Attacks, unblocked combat damage, and the 0-life loss

**Files:**
- Modify: `crates/mtgml-rules/src/turn_progression.rs`:
  - `install_attacker_request` (:1040);
  - `validate_magic_pending_request` (:121-174, domain `{0, n}`);
  - `execute_magic_response` (:80-90, `Answer::Attackers(Vec<GameObjectId>)`);
  - `progress` (tap the attackers, `ObjectTapped`; blockers `None` per
    attacker);
  - `advance`: the DeclareBlockers and CombatDamage arms (:508-510) and
    704.5a after damage;
  - `validate_slice` (:261-265, allow attackers with no blockers). It also
    rejects a running state in which a player who has not lost is at life
    ≤ 0. That replaces the "no state-based action can apply" assumption
    (:242-244) with an assertion that no SBA is pending at a decision
    boundary (spec §4).
- Modify: `crates/mtgml-state/src/validation/core.rs:54`: remove the
  8-attacker cap.
- Modify: `crates/mtgml-rules/src/events.rs`:
  - CombatDamageDealt projection (:1438: player recipients must match the life
    change; creature recipients fail closed until 3a);
  - `is_projectable_public_source_event` (:927): add `LifeChanged` and
    `AttackersDeclared`.
- Modify: `crates/mtgml-observation/src/observed_event_v4.rs`: new
  `AttackersDeclared { attacking_player, defending_player, attackers: Vec<OpaqueObjectId> }`.
- Modify: `crates/mtgml-environment/src/successor_projection.rs`: arms for
  `LifeChanged` and `AttackersDeclared`.
- Python `_events_v4.py`, `schemas/observed-event-envelope.v4.schema.json`,
  wire goldens `observed-event-v4-attackers-declared.json` and
  `observed-event-v4-life-changed.json`, plus the manifest.
- Registry: `rules/summoning-sickness`, `rules/combat-damage`,
  `rules/damage-and-life` and `rules/state-based-actions-combat` become
  `covered` for unblocked damage.
- Test: `crates/mtgml-environment/tests/creature_game.rs`, and
  `crates/mtgml-state/src/tests/` for the cap.

**Interfaces:**
- Consumes:
  - `PermanentsState` (Task 4);
  - `S1QueryAuthority` base power from `derive_base_characteristics`
    (`base_power_toughness`).
- **Rules:**
  - A creature can attack iff it is untapped, its controller is active, and
    `controlled_since_turn < core.turn_number` (CR 302.6).
  - Attackers tap (508.1f).
  - **DeclareBlockers** with attackers: if the defending player controls an
    untapped creature, return `Err(TurnProgressUnsupported)` (blocks arrive in
    3a). Otherwise no request; priority opens in the step.
  - **CombatDamage:** each attacker deals its power to the defending player
    (510.1a; power 0 deals none). This emits `CombatDamageDealt`,
    `LifeChanged` and `CombatDamageStepCompleted` and calls
    `record_life_loss`. Then 704.5a: life ≤ 0 means `has_lost` and
    `GameOver { loser }`; otherwise the active player gets priority (510.3).
  - **Ruling:** `DamageApplied` is not emitted, and its deletion moves to 3a,
    where the damage family is unified (spec §4). Cost: one unused event kind
    for one more plan.

- [ ] **Step 1: Write the failing tests:**
  - `a_creature_cannot_attack_the_turn_it_arrives_but_can_on_its_controllers_next_turn`.
  - `an_unblocked_attack_lowers_the_defenders_life`: Hill Giant attacks; the
    defender goes 20 → 17, and both players observe `life_changed`.
  - `attackers_tap_and_both_players_see_the_attack`.
  - `zero_life_ends_the_game`: terminal status `RulesLoss`; further submits
    give `EpisodeClosed`; the replay is equal.
  - `overkill_damage_shows_negative_life_and_ends_the_game`.
  - `a_defender_with_an_untapped_creature_fails_closed`: a synthetic state
    where the defender controls an untapped Lions gives
    `TurnProgressUnsupported` at DeclareBlockers.
  - State: nine attackers validate.
  - Slice: a decision state with a player at 0 life who has not lost is
    rejected.
- [ ] **Step 2: Run** them. Expected: FAIL (`TurnProgressUnsupported` at
  DeclareAttackers with a candidate).
- [ ] **Step 3: Implement** as specified.
- [ ] **Step 4: Run** the workspace tests, the Python suite and the release
  smoke. Expected: PASS, and the land pins are unchanged (attacker domain
  `{0, 0}` with no candidates, as today).
- [ ] **Step 5: Commit** `feat: creatures attack and an unblocked attack can end the game`.

### Task 8: The observation shows creatures and attackers

**Files:**
- Modify: `crates/mtgml-observation/src/magic_observation.rs` and
  `magic_shared_execution_observation_v1.rs`.
- Modify: `crates/mtgml-environment/src/player_projection.rs`
  (`project_magic_basic_land_observation`, which has the catalog).
- Python: `_magic_basic_land_observation_v1.py`,
  `magic_shared_execution_observation_v1.py`, `observation.py` and
  `__init__.py`.
- Schemas, examples and all observation schema and wire negatives (each must
  still fail only for its own reason; check with jsonschema `iter_errors`),
  plus the observation wire golden.
- `docs/INFORMATION_MODEL.md` §Current player products.
- Test: `crates/mtgml-environment/tests/creature_game.rs`,
  `crates/mtgml-observation/src/tests.rs`, and
  `python/tests/test_public_player_state.py`.

**Interfaces:**
- Produces `creatures: Vec<CreatureObservationV1 { object: OpaqueObjectId, controller: PlayerId, power: i64, toughness: i64, controlled_since_turn: u64 }>`,
  strictly ascending by `object`, and `attacking: Vec<OpaqueObjectId>`,
  strictly ascending.
  - P/T is printed P/T from the catalog face.
  - The projection fails closed (`ServiceUnavailable`) if any temporary effect
    exists.
  - **Ruling:** the raw public fact `controlled_since_turn`, not a derived
    "can attack" flag. That leaves no derived semantics to get wrong.

- [ ] **Step 1: Write the failing tests:**
  - `both_players_see_the_creature_with_its_power_toughness_and_arrival_turn`.
  - `both_players_see_which_creatures_attack`.
  - DTO validation rejects unordered rows and `attacking` ids that are not
    creatures. Python mirrors the same cases.
- [ ] **Step 2: Run** them. Expected: FAIL to compile or decode.
- [ ] **Step 3: Implement**, following commit `b58292e2` as the template for
  every file.
- [ ] **Step 4: Run** everything. Expected: PASS after re-pinning the smoke
  fingerprints. The land games' scratch fingerprint over actor, response and
  checkpoint digest is unchanged.
- [ ] **Step 5: Commit** `feat: observe creatures and attackers`.

### Task 9: Asymmetric smoke games

**Files:**
- Modify: `crates/mtgml-environment/tests/random_smoke.rs`.
- Modify: `README.md` (Production turn loop / Real Magic semantics lines).
- Modify: the capability docs' conformance lists.

**Interfaces:**
- Consumes: `creature_deck_game` (Task 3).
- Asymmetric deck: 17 random lands plus 10 random creatures out of the three
  witnesses, against 27 random lands. The creature deck belongs to P1 for even
  seeds and to P2 for odd seeds.

- [ ] **Step 1: Write the failing test**
  `asymmetric_games_cast_attack_and_end_at_zero_life` (release; ignored in
  debug):
  - Seeds `FIRST_SEED..FIRST_SEED+10`, until turn 31 or a terminal status.
  - Each game is deterministic (a re-run is equal) and replays equal.
  - Over all games:
    - at least one `SpellCast` (count accepted CastSpell answers);
    - at least one ManaPayment decision;
    - at least one non-empty attacker declaration;
    - at least one game that ends in `Terminal { RulesLoss }` with the land
      player at life ≤ 0.

  Add a debug-build short pin `ASYMMETRIC_SHORT_FINGERPRINT` for
  `FIRST_SEED`, played until turn 6 begins.
- [ ] **Step 2: Run** it. Expected: FAIL (the pin is missing).
- [ ] **Step 3: Implement** the decks and counters, then pin the printed value.
  The thirty-turn land test stays unchanged. If any coverage count is zero,
  raise the creature count, not the seed set, and ledger it.
- [ ] **Step 4: Run** the full integration gate
  (`.venv/Scripts/python.exe scripts/run_checks.py integration`). The
  worktree needs a `.venv` junction to the main checkout's venv. Expected:
  every step PASS.
- [ ] **Step 5: Commit** `test: random smoke games pit a creature deck against a land deck`.
