//! Shared production-path fixtures: a two-player basic-land game built the
//! way a real game starts (hidden libraries, owner-known hands, empty
//! battlefield), run through the production V8 controller.

#![allow(dead_code)]

use mtgml_card_ir::{
    admit_executable_profile_v1, decode_content_manifest_v1, CardSemanticBindingV1,
    ExecutableProfileAdmissionV1,
};
use mtgml_environment::{EnvironmentCheckpointV8, TrustedEnvironmentController};
use mtgml_model::{
    CapabilityRequirementV1, CardDefinitionId, ExecutionIdentityV1, ExecutionProgramV1,
    GameObjectId, OpaqueObjectId, PhysicalCardId, PlayerId, RulesAuthorityV1,
    RulesContractManifestV1, SemanticContractManifestV1, ZoneKind,
};
use mtgml_replay::{
    ContentContractMaterialV1, InitialEnvironmentIdentityV8, ReplayManifestV8,
    SemanticContractMaterialV7,
};
use mtgml_state::{
    EngineState, GameObject, KnowledgeAcquisitionReason, KnowledgeRecordV2, KnownLocationFactV2,
    PriorityState, TurnPosition, VisibilityPartition, ZoneLocation, ZonePosition,
};

pub const P1: PlayerId = PlayerId(1);
pub const P2: PlayerId = PlayerId(2);

pub const CONTENT: &[u8] =
    include_bytes!("../../../../cards/definitions/basic-land-v1/content-contract.v1.cbor");
pub const PROVENANCE: &[u8] =
    include_bytes!("../../../../cards/definitions/basic-land-v1/provenance.v1.cbor");
pub const RULES_SNAPSHOT: &str = "wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca";

/// The admission of a complete two-player game: Mountain and Plains plus the
/// game-rule roots.
pub fn game_admission() -> ExecutableProfileAdmissionV1 {
    let content_id =
        mtgml_persistence::content_contract_digest::calculate_content_contract_id_v1(CONTENT)
            .unwrap();
    let closure = [
        "rules/basic-land-mana",
        "rules/basic-priority",
        "rules/cleanup-reset",
        "rules/combat-phase",
        "rules/declare-attackers",
        "rules/draw-card",
        "rules/land-play",
        "rules/mana-pool",
        "rules/state-based-actions-combat",
        "rules/state-based-actions-empty-library",
        "rules/turn-structure",
        "rules/zone-incarnation",
    ]
    .into_iter()
    .map(|key| CapabilityRequirementV1 {
        key: key.to_owned(),
        version: "0.1.0".to_owned(),
    })
    .collect();
    let rules = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: RULES_SNAPSHOT.to_owned(),
        },
        capability_closure: Some(closure),
    };
    let semantic = SemanticContractManifestV1 {
        rules_contract_id:
            mtgml_persistence::semantic_contract_digest::calculate_rules_contract_id_v1(&rules)
                .unwrap(),
        format_contract_id: None,
        content_contract_id: Some(content_id.clone()),
    };
    let execution = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id:
            mtgml_persistence::semantic_contract_digest::calculate_semantic_contract_id_v1(
                &semantic,
            )
            .unwrap(),
    };
    admit_executable_profile_v1(
        CONTENT,
        &content_id,
        PROVENANCE,
        &rules,
        &semantic,
        &execution,
    )
    .unwrap()
}

/// (Mountain, Plains) definition ids of the admitted content.
pub fn land_definitions() -> (CardDefinitionId, CardDefinitionId) {
    let manifest = decode_content_manifest_v1(CONTENT).unwrap();
    let find = |subtype| {
        manifest
            .definitions
            .iter()
            .find(|definition| {
                matches!(definition.semantic_binding,
                    CardSemanticBindingV1::ProfiledV1 { body, .. } if body.subtype == subtype)
            })
            .unwrap()
            .card_definition_id
    };
    (
        find(mtgml_card_ir::BasicLandSubtypeV1::Mountain),
        find(mtgml_card_ir::BasicLandSubtypeV1::Plains),
    )
}

/// Deterministic test PRNG (SplitMix64); independent of the engine RNG.
pub struct SplitMix64(pub u64);

impl SplitMix64 {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..bound` (`bound > 0`).
    pub fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}

/// Mountain/Plains cards chosen by `seed`.
pub fn random_lands(count: usize, rng: &mut SplitMix64) -> Vec<CardDefinitionId> {
    let (mountain, plains) = land_definitions();
    (0..count)
        .map(|_| if rng.below(2) == 0 { mountain } else { plains })
        .collect()
}

/// A two-player game at P1's first precombat main: `library_size` hidden
/// cards and `hand_size` owner-known cards per player, chosen by `seed`.
pub fn two_player_land_game(
    library_size: usize,
    hand_size: usize,
    seed: u64,
) -> TrustedEnvironmentController {
    let mut rng = SplitMix64(seed);
    let libraries = [
        random_lands(library_size, &mut rng),
        random_lands(library_size, &mut rng),
    ];
    let hands = [
        random_lands(hand_size, &mut rng),
        random_lands(hand_size, &mut rng),
    ];
    land_game(&libraries, &hands, seed)
}

/// As `two_player_land_game` with explicit libraries (index 0 is the top)
/// and hands for P1 and P2.
pub fn land_game(
    libraries: &[Vec<CardDefinitionId>; 2],
    hands: &[Vec<CardDefinitionId>; 2],
    seed: u64,
) -> TrustedEnvironmentController {
    try_land_game(libraries, hands, seed).unwrap()
}

/// As `land_game`, returning the error if the game cannot be created.
pub fn try_land_game(
    libraries: &[Vec<CardDefinitionId>; 2],
    hands: &[Vec<CardDefinitionId>; 2],
    seed: u64,
) -> Result<TrustedEnvironmentController, mtgml_environment::ControllerError> {
    let admission = game_admission();
    let mut state = land_game_state(libraries, hands, seed);
    let status = mtgml_model::EpisodeStatus::Running;
    mtgml_rules::install_basic_land_request(&admission, &mut state, P1, &status).unwrap();
    let checkpoint = EnvironmentCheckpointV8::new_for_basic_land_profile(
        &admission,
        state.clone(),
        status.clone(),
        Default::default(),
        admission.execution_identity().clone(),
    )?;
    let runtime = mtgml_environment::BasicLandEnvironmentRuntimeV8::new(
        admission.clone(),
        state,
        status,
        Default::default(),
        replay_manifest(&admission, &checkpoint),
    )?;
    Ok(TrustedEnvironmentController::new(runtime))
}

fn land_game_state(
    libraries: &[Vec<CardDefinitionId>; 2],
    hands: &[Vec<CardDefinitionId>; 2],
    seed: u64,
) -> EngineState {
    let mut setup = mtgml_state::SyntheticV4Setup::synthetic_compatibility();
    setup.position = TurnPosition::PrecombatMain;
    setup.priority = PriorityState::HeldBy {
        player: P1,
        consecutive_passes: 0,
    };
    let mut root_seed = [0_u8; 32];
    root_seed[..8].copy_from_slice(&seed.to_le_bytes());
    // The builder's state has one empty mana pool and turn history per
    // player and no pending request.
    let mut parts =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [P1, P2],
            root_seed: mtgml_random::RootSeed256(root_seed),
            setup,
        })
        .unwrap();
    // The synthetic reset seeds a public battlefield object and a face-down
    // library object; a real game starts without either.
    for object in [GameObjectId(1), GameObjectId(2)] {
        parts.zones.objects.remove(&object);
        parts.zones.locations.remove(&object);
        for members in parts.zones.ordered_zones.values_mut() {
            members.retain(|member| *member != object);
        }
        for (player, identity) in &mut parts.perspective_identities.players {
            if let Some(opaque) = identity.object_to_opaque.remove(&object) {
                identity.opaque_to_object.remove(&opaque);
                parts
                    .knowledge
                    .players
                    .get_mut(player)
                    .unwrap()
                    .active
                    .remove(&opaque);
            }
        }
    }
    parts
        .zones
        .ordered_zones
        .retain(|_, members| !members.is_empty());
    let mut state = parts;
    for (player, library) in [P1, P2].into_iter().zip(libraries) {
        for definition in library {
            add_card(&mut state, player, *definition, ZoneKind::Library);
        }
    }
    for (player, hand) in [P1, P2].into_iter().zip(hands) {
        for definition in hand {
            add_card(&mut state, player, *definition, ZoneKind::Hand);
        }
    }
    state.validate_structure().unwrap();
    state
}

/// A library card is hidden from everyone; a hand card is known to its
/// owner only.
fn add_card(
    state: &mut EngineState,
    owner: PlayerId,
    definition: CardDefinitionId,
    zone: ZoneKind,
) {
    let parts = &mut *state;
    let id = parts.allocators.next_object_id;
    parts.allocators.next_object_id = GameObjectId(id.0 + 1);
    let physical_card = Some(PhysicalCardId(1_000 + id.0));
    parts.zones.objects.insert(
        id,
        GameObject {
            id,
            physical_card,
            card_definition: definition,
            owner,
            controller: owner,
            tapped: false,
            face_down: false,
        },
    );
    let location = if zone == ZoneKind::Library {
        let base = ZoneLocation {
            zone,
            player: Some(owner),
            position: ZonePosition::Top { offset: 0 },
            visibility: VisibilityPartition::FaceDown,
            partition: None,
        };
        let order = parts.zones.ordered_zones.entry(base.key()).or_default();
        let location = ZoneLocation {
            position: ZonePosition::Top {
                offset: order.len() as u32,
            },
            ..base
        };
        order.push(id);
        location
    } else {
        let location = ZoneLocation {
            zone,
            player: Some(owner),
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::OwnerOnly,
            partition: None,
        };
        let identity = parts
            .perspective_identities
            .players
            .get_mut(&owner)
            .unwrap();
        let opaque = identity.next_opaque_object_id;
        identity.next_opaque_object_id = OpaqueObjectId(opaque.0 + 1);
        identity.object_to_opaque.insert(id, opaque);
        identity.opaque_to_object.insert(opaque, id);
        parts
            .knowledge
            .players
            .get_mut(&owner)
            .unwrap()
            .active
            .insert(
                opaque,
                KnowledgeRecordV2 {
                    opaque_object: opaque,
                    physical_card,
                    card_definition: Some(definition),
                    known_location: Some(KnownLocationFactV2 {
                        location: location.clone(),
                        provenance: KnowledgeAcquisitionReason::InitialConfiguration,
                    }),
                    acquisition: KnowledgeAcquisitionReason::InitialConfiguration,
                    historical_locations: Vec::new(),
                },
            );
        location
    };
    parts.zones.locations.insert(id, location);
    state.card_rules.faces.faces.insert(id, 0);
}

pub fn checkpoint_identity(checkpoint: &EnvironmentCheckpointV8) -> InitialEnvironmentIdentityV8 {
    InitialEnvironmentIdentityV8 {
        state_revision: checkpoint.state.revision,
        full_state_digest: checkpoint.state_digest.clone(),
        episode_status: checkpoint.status.clone(),
        environment_limit_counters: checkpoint.limit_counters.clone(),
        checkpoint_codec_identity: checkpoint.codec.clone(),
        checkpoint_digest: checkpoint.checkpoint_digest.clone(),
        execution_identity: checkpoint.execution_identity.clone(),
    }
}

pub fn replay_manifest(
    admission: &ExecutableProfileAdmissionV1,
    checkpoint: &EnvironmentCheckpointV8,
) -> ReplayManifestV8 {
    let mut manifest: ReplayManifestV8 = serde_json::from_str(include_str!(
        "../../../../schemas/examples/replay-manifest-v8.json"
    ))
    .unwrap();
    manifest.execution_identity = admission.execution_identity().clone();
    manifest.semantic_contract = SemanticContractMaterialV7 {
        semantic_contract_id: admission.semantic_contract_id().clone(),
        manifest: admission.semantic_contract_manifest().clone(),
        rules_manifest: admission.rules_contract_manifest().clone(),
        content_contract: Some(
            ContentContractMaterialV1::from_manifest(decode_content_manifest_v1(CONTENT).unwrap())
                .unwrap(),
        ),
    };
    let mut second_deck = manifest.decks[0].clone();
    second_deck.player = P2;
    second_deck.deck_id = "deck:synthetic-p2".to_owned();
    manifest.decks.push(second_deck);
    manifest.rules_snapshot = RULES_SNAPSHOT.to_owned();
    manifest.card_bundle = admission.content_contract_id().to_string();
    manifest.randomness.root_seed_hex = checkpoint.state.random.root_seed.to_lower_hex();
    manifest.initial_identity = checkpoint_identity(checkpoint);
    manifest
}
