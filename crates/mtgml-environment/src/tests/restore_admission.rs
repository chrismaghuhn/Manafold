use crate::checkpoint::{
    CheckpointValidationError, EnvironmentCheckpointV6, CHECKPOINT_CODEC_ID_V6,
    CHECKPOINT_CODEC_SEMANTIC_VERSION_V6,
};
use crate::semantic_catalog::{
    admit_restore, CatalogEntry, RestoreAdmissionError, RuntimeSemanticCatalog,
};
use crate::semantic_catalog_generated::{
    magic_turn_structure_0_1_0_semantic_contract_id,
    synthetic_legacy_default_rules_manifest,
    synthetic_legacy_default_semantic_manifest,
};
use mtgml_decision::{
    AuthoritativeCandidateV2, AuthoritativeDecisionRequestV2, CandidateIntent, DecisionDomainV2,
    DecisionVisibility, EngineCandidateBinding,
};
use mtgml_model::{
    CapabilityRequirementV1, CardDefinitionId, CheckpointCodecIdentity, ContinuationId,
    EnvironmentLimitCounters, ExecutionIdentityV1, ExecutionProgramV1, EpisodeStatus,
    FullStateDigestV5, GameObjectId, OpaqueObjectId, PhysicalCardId, PlayerDecisionIdV1,
    PlayerId, RulesAuthorityV1, RulesContractManifestV1, SemanticContractIdV1,
    SemanticContractManifestV1, StateRevision, ZoneKind,
};
use mtgml_persistence::semantic_contract_digest::{
    calculate_rules_contract_id_v1, calculate_semantic_contract_id_v1,
};
use mtgml_state::{
    construct_synthetic_engine_state, BaseCharacteristics, ContinuationPayloadV2,
    ContinuationRecordV2, ControlHistory, EngineState, FoundationCreatureSource,
    FoundationSourceKind, GameObject, KnownLocationFactV2, KnowledgeAcquisitionReason,
    KnowledgeRecordV2, PendingDecisionRecordV2, SbaObjectCauseV1, SbaSelectedActionV1,
    SyntheticResetInputs, SyntheticV4Setup, VisibilityPartition, ZoneLocation, ZonePosition,
};

// === Helpers ===

fn synthetic_identity() -> ExecutionIdentityV1 {
    ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: synthetic_legacy_default_semantic_contract_id(),
    }
}

fn v6_codec() -> CheckpointCodecIdentity {
    CheckpointCodecIdentity {
        codec_id: CHECKPOINT_CODEC_ID_V6.to_string(),
        semantic_version: CHECKPOINT_CODEC_SEMANTIC_VERSION_V6.to_string(),
    }
}

fn valid_v6_checkpoint(identity: ExecutionIdentityV1) -> EnvironmentCheckpointV6 {
    let bk = backend();
    let v4 = bk.checkpoint().unwrap();
    EnvironmentCheckpointV6::new(
        v4.state,
        v4.status,
        v4.limit_counters,
        v6_codec(),
        identity,
    )
    .unwrap()
}

// A state that passes generic validate_engine_state but fails the synthetic
// program's runtime validation: a standalone ChooseNumber decision (not a
// valid synthetic entry).
fn synthetic_incompatible_state() -> EngineState {
    let mut state =
        construct_synthetic_engine_state(SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: seed(),
            setup: SyntheticV4Setup::m2_compatibility(),
        })
        .unwrap();
    state.execution.pending_decision = Some(PendingDecisionRecordV2 {
        request: AuthoritativeDecisionRequestV2 {
            decision_id: mtgml_model::DecisionId(1),
            player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: PlayerId(1),
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseNumber {
                minimum: 0,
                maximum: 3,
            },
            candidates: Vec::new(),
            continuation_id: None,
        },
    });
    // The generic state validator passes; the synthetic kernel rejects it.
    mtgml_state::validate_engine_state(&state).unwrap();
    state
}

// === Phase 1: checkpoint structural validation ===

#[test]
fn invalid_checkpoint_rejects_at_structural_validation() {
    let catalog = RuntimeSemanticCatalog::production();
    let mut checkpoint = valid_v6_checkpoint(synthetic_identity());
    // Corrupt the checkpoint digest so digest recompute (phase 2) rejects.
    checkpoint.checkpoint_digest = CheckpointDigestV6::from_digest_bytes([0xee; 32]);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::CheckpointValidation(CheckpointValidationError::CheckpointDigest)
    );
}

// === Exact S1 positive and negative restore admission ===

fn magic_identity() -> ExecutionIdentityV1 {
    ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: magic_turn_structure_0_1_0_semantic_contract_id(),
    }
}

fn s1_valid_state() -> EngineState {
    let mut state =
        construct_synthetic_engine_state(SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: mtgml_random::RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
            setup: SyntheticV4Setup {
                position: mtgml_state::TurnPosition::Beginning {
                    step: mtgml_state::BeginningStep::Untap,
                },
                priority: mtgml_state::PriorityState::None,
                combat: None,
                foundation_sources: std::collections::BTreeMap::new(),
            },
        })
        .unwrap();
    state.execution.pending_decision = None;
    state
}

#[test]
fn current_magic_s1_contract_rejects_restore_of_magic_sba_continuation() {
    let state = magic_sba_continuation_state();
    let checkpoint = EnvironmentCheckpointV6::new(
        state,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        v6_codec(),
        magic_identity(),
    )
    .unwrap();
    let before = checkpoint.clone();
    let checkpoint_roundtrip = EnvironmentCheckpointV6::new(
        checkpoint.state.clone(),
        checkpoint.status.clone(),
        checkpoint.limit_counters.clone(),
        checkpoint.codec.clone(),
        checkpoint.execution_identity.clone(),
    )
    .unwrap();
    assert_eq!(checkpoint_roundtrip, checkpoint);
    checkpoint_roundtrip.validate().unwrap();

    assert_eq!(
        admit_restore(&RuntimeSemanticCatalog::production(), &checkpoint),
        Err(RestoreAdmissionError::ProgramStateIncompatible)
    );
    assert_eq!(checkpoint, before);
    checkpoint.validate().unwrap();
}

pub(super) fn magic_sba_continuation_state() -> EngineState {
    let mut state = s1_valid_state();
    state.revision = StateRevision(1);
    state.core.position = mtgml_state::TurnPosition::Beginning {
        step: mtgml_state::BeginningStep::Upkeep,
    };
    let object = GameObjectId(3);
    let location = ZoneLocation {
        zone: ZoneKind::Battlefield,
        player: None,
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
        partition: None,
    };
    state.zones.objects.insert(
        object,
        GameObject {
            id: object,
            physical_card: Some(PhysicalCardId(3)),
            card_definition: CardDefinitionId(3),
            owner: PlayerId(1),
            controller: PlayerId(1),
            tapped: false,
            face_down: false,
        },
    );
    state.zones.locations.insert(object, location.clone());
    state.allocators.next_object_id = GameObjectId(4);

    for object_id in [GameObjectId(1), object] {
        state.foundation_sources.insert(
            object_id,
            FoundationCreatureSource {
                source_kind: FoundationSourceKind::Creature,
                base_characteristics: BaseCharacteristics::Simple {
                    power: 2,
                    toughness: 0,
                },
                marked_damage: 0,
                control_history: ControlHistory::BeforeTurnStart { turn_number: 1 },
            },
        );
    }

    for (player, opaque) in [(PlayerId(1), OpaqueObjectId(2)), (PlayerId(2), OpaqueObjectId(3))] {
        let identity = state
            .perspective_identities
            .players
            .get_mut(&player)
            .unwrap();
        identity.opaque_to_object.insert(opaque, object);
        identity.object_to_opaque.insert(object, opaque);
        identity.next_opaque_object_id = OpaqueObjectId(opaque.0 + 1);
        state
            .knowledge
            .players
            .get_mut(&player)
            .unwrap()
            .active
            .insert(
                opaque,
                KnowledgeRecordV2 {
                    opaque_object: opaque,
                    physical_card: Some(PhysicalCardId(3)),
                    card_definition: Some(CardDefinitionId(3)),
                    known_location: Some(KnownLocationFactV2 {
                        location: location.clone(),
                        provenance: KnowledgeAcquisitionReason::InitialConfiguration,
                    }),
                    historical_locations: Vec::new(),
                    acquisition: KnowledgeAcquisitionReason::InitialConfiguration,
                },
            );
    }

    state
        .execution
        .continuations
        .insert(
            ContinuationId(1),
            ContinuationRecordV2 {
                id: ContinuationId(1),
                actor: PlayerId(1),
                created_at_revision: StateRevision(1),
                stage_index: 0,
                payload: ContinuationPayloadV2::MagicSbaGraveyardOrderV1 {
                    round_start_revision: StateRevision(0),
                    selected_sba_actions: vec![
                        SbaSelectedActionV1::ObjectToOwnerGraveyard {
                            object: GameObjectId(1),
                            causes: vec![SbaObjectCauseV1::ZeroToughness],
                        },
                        SbaSelectedActionV1::ObjectToOwnerGraveyard {
                            object,
                            causes: vec![SbaObjectCauseV1::ZeroToughness],
                        },
                    ],
                    apnap_owners: vec![PlayerId(1)],
                    next_owner_index: 0,
                    completed_owner_orders: Vec::new(),
                },
            },
        );
    state.allocators.next_continuation_id = ContinuationId(2);
    state.execution.pending_decision = Some(PendingDecisionRecordV2 {
        request: AuthoritativeDecisionRequestV2 {
            decision_id: mtgml_model::DecisionId(1),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(1),
            actor: PlayerId(1),
            visibility: DecisionVisibility::ActingPlayerOnly,
            decision: DecisionDomainV2::Order {
                minimum: 2,
                maximum: 2,
            },
            candidates: [GameObjectId(1), object]
                .into_iter()
                .enumerate()
                .map(|(index, object)| {
                    let opaque = if object == GameObjectId(1) {
                        OpaqueObjectId(1)
                    } else {
                        OpaqueObjectId(2)
                    };
                    AuthoritativeCandidateV2 {
                        candidate_id: mtgml_model::CandidateIdV1(index as u32),
                        visible_intent: CandidateIntent::SelectObject { object: opaque },
                        trusted_binding: EngineCandidateBinding::SelectObject { object },
                    }
                })
                .collect(),
            continuation_id: Some(ContinuationId(1)),
        },
    });
    mtgml_state::validate_engine_state(&state).unwrap();
    state
}

fn s1_checkpoint(identity: ExecutionIdentityV1) -> EnvironmentCheckpointV6 {
    let state = s1_valid_state();
    let codec = CheckpointCodecIdentity {
        codec_id: CHECKPOINT_CODEC_ID_V6.to_string(),
        semantic_version: CHECKPOINT_CODEC_SEMANTIC_VERSION_V6.to_string(),
    };
    EnvironmentCheckpointV6::new(
        state,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        codec,
        identity,
    )
    .unwrap()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RestoreAdmissionFingerprint {
    checkpoint: EnvironmentCheckpointV6,
    replay: mtgml_replay::AuthoritativeReplayV6,
    player_bytes: Vec<Vec<u8>>,
}

fn capture_restore_admission_fingerprint(
    controller: &TrustedEnvironmentController,
) -> RestoreAdmissionFingerprint {
    let player_bytes = [PlayerId(1), PlayerId(2)]
        .into_iter()
        .map(|player| {
            let endpoint = controller.bind_player(player).unwrap();
            let mut bytes = Vec::new();
            bytes.extend(mtgml_wire::encode_canonical(&endpoint.observation().unwrap()).unwrap());
            bytes.extend(
                mtgml_wire::encode_canonical(&endpoint.information_state().unwrap()).unwrap(),
            );
            if let Some(decision) = endpoint.visible_decision().unwrap() {
                bytes.push(1);
                bytes.extend(mtgml_wire::encode_canonical(&decision).unwrap());
            } else {
                bytes.push(0);
            }
            bytes
        })
        .collect();
    RestoreAdmissionFingerprint {
        checkpoint: controller.checkpoint().unwrap(),
        replay: controller.export_replay().unwrap(),
        player_bytes,
    }
}

fn assert_controller_restore_rejection_is_nonmutating(
    label: &str,
    controller: &TrustedEnvironmentController,
    checkpoint: EnvironmentCheckpointV6,
    catalog: RuntimeSemanticCatalog,
    expected: impl FnOnce(&ControllerError) -> bool,
) {
    let before = capture_restore_admission_fingerprint(controller);
    let error = controller
        .restore_with_catalog(checkpoint, catalog)
        .expect_err("restore admission must reject");
    assert!(expected(&error), "{label}: unexpected error {error:?}");
    let after = capture_restore_admission_fingerprint(controller);
    assert_eq!(after, before, "{label}: restore mutated the complete fingerprint");
}

fn valid_wrong_closure_catalog() -> (RuntimeSemanticCatalog, SemanticContractIdV1, SemanticContractIdV1) {
    let rules_manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f".into(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/turn-structure".into(),
            version: "0.2.0".into(),
        }]),
    };
    let wrong_rules_id = calculate_rules_contract_id_v1(&rules_manifest).unwrap();
    let semantic_manifest = SemanticContractManifestV1 {
        rules_contract_id: wrong_rules_id,
        format_contract_id: None,
        content_contract_id: None,
    };
    let wrong_semantic_id = calculate_semantic_contract_id_v1(&semantic_manifest).unwrap();
    let catalog = RuntimeSemanticCatalog::from_entries(vec![CatalogEntry {
        semantic_contract_id: wrong_semantic_id.clone(),
        manifest: semantic_manifest,
        rules_manifest,
    }]);
    (
        catalog,
        wrong_semantic_id,
        magic_turn_structure_0_1_0_semantic_contract_id(),
    )
}

#[test]
fn exact_turn_structure_restore_admits() {
    // Exact S1 contract + valid S1 state + MagicRules identity.
    let catalog = RuntimeSemanticCatalog::production();
    let checkpoint = s1_checkpoint(magic_identity());
    let result = admit_restore(&catalog, &checkpoint);
    assert!(result.is_ok(), "exact S1 restore must admit");
}

#[test]
fn exact_turn_structure_invalid_state_rejected() {
    // MagicRules + exact S1 + state with pending decision.
    // Passes generic validate_engine_state but fails
    // validate_turn_structure_support (pending decision), so
    // phase 8 rejects before backend construction.
    let catalog = RuntimeSemanticCatalog::production();
    let state = synthetic_incompatible_state();
    let codec = v6_codec();
    let identity = magic_identity();
    let checkpoint = EnvironmentCheckpointV6::new(
        state,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        codec,
        identity,
    )
    .unwrap();
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::ProgramStateIncompatible
    );
}

#[test]
fn exact_turn_structure_synthetic_contract_rejected() {
    // MagicRules + synthetic contract → ProgramAuthorityMismatch.
    let catalog = RuntimeSemanticCatalog::production();
    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: synthetic_legacy_default_semantic_contract_id(),
    };
    let checkpoint = s1_checkpoint(identity);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::ProgramAuthorityMismatch
    );
}

#[test]
fn exact_turn_structure_arbitrary_cr_contract_rejected() {
    // MagicRules + arbitrary ComprehensiveRules contract → SemanticContractUnknown.
    // The arbitrary contract is NOT in the catalog, so phase 3 (resolve) fails
    // before phase 7 (runtime support). Preserves frozen error precedence.
    let catalog = RuntimeSemanticCatalog::production();
    let cr_rules_manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "wotc-cr-2026-08-07-txt-20260819-sha256-different-snapshot-000000000000000000000000000000000000000000000000".to_string(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/turn-structure".to_string(),
            version: "0.1.0".to_string(),
        }]),
    };
    let cr_rules_id = calculate_rules_contract_id_v1(&cr_rules_manifest).unwrap();
    let cr_manifest = SemanticContractManifestV1 {
        rules_contract_id: cr_rules_id,
        format_contract_id: None,
        content_contract_id: None,
    };
    let cr_semantic_id = calculate_semantic_contract_id_v1(&cr_manifest).unwrap();

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: cr_semantic_id,
    };
    let checkpoint = s1_checkpoint(identity);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::SemanticContractUnknown
    );
}

#[test]
fn exact_turn_structure_unknown_contract_rejected() {
    // MagicRules + unknown contract → SemanticContractUnknown.
    let catalog = RuntimeSemanticCatalog::production();
    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: SemanticContractIdV1::from_digest_bytes([0u8; 32]),
    };
    let checkpoint = s1_checkpoint(identity);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::SemanticContractUnknown
    );
}

#[test]
fn controller_restore_exact_turn_structure_nonmutation_on_rejection() {
    // Controller-level nonmutation: rejected restore must not mutate
    // checkpoint, replay, state, execution identity, status, or limit counters.
    let controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: synthetic_legacy_default_semantic_contract_id(),
    };
    let checkpoint = valid_v6_checkpoint(identity);
    let result = controller.restore(checkpoint);
    assert!(matches!(
        result,
        Err(ControllerError::ProgramAuthorityMismatch)
    ));
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), before_replay);
}

#[test]
fn turn_structure_task11_program_authority_pairing_both_directions_is_nonmutating() {
    let controller = TrustedEnvironmentController::new(backend());

    let magic_with_synthetic = valid_v6_checkpoint(ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: synthetic_legacy_default_semantic_contract_id(),
    });
    assert_controller_restore_rejection_is_nonmutating(
        "MagicRules plus SyntheticLegacy",
        &controller,
        magic_with_synthetic,
        RuntimeSemanticCatalog::production(),
        |error| matches!(error, ControllerError::ProgramAuthorityMismatch),
    );

    let synthetic_with_magic = s1_checkpoint(ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: magic_turn_structure_0_1_0_semantic_contract_id(),
    });
    assert_controller_restore_rejection_is_nonmutating(
        "SyntheticRulesCompat plus ComprehensiveRules",
        &controller,
        synthetic_with_magic,
        RuntimeSemanticCatalog::production(),
        |error| matches!(error, ControllerError::ProgramAuthorityMismatch),
    );
}

#[test]
fn tampered_checkpoint_digest_rejects_at_structural_validation() {
    let catalog = RuntimeSemanticCatalog::production();
    let mut checkpoint = valid_v6_checkpoint(synthetic_identity());
    checkpoint.checkpoint_digest = mtgml_model::CheckpointDigestV6::from_digest_bytes([0xee; 32]);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::CheckpointValidation(CheckpointValidationError::CheckpointDigest)
    );
}

// === Phase 3: unknown semantic contract ===

#[test]
fn unknown_semantic_contract_id_rejected() {
    let catalog = RuntimeSemanticCatalog::production();
    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: SemanticContractIdV1::from_digest_bytes([0u8; 32]),
    };
    let checkpoint = valid_v6_checkpoint(identity);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::SemanticContractUnknown
    );
}

// === Phase 4: semantic contract digest mismatch ===

#[test]
fn semantic_contract_digest_mismatch_rejected() {
    // Test-only catalog: entry has a wrong semantic_contract_id that does not
    // recompute from its manifest. The catalog is a frozen data table; the
    // admission function performs the recompute-and-compare.
    let wrong_id = SemanticContractIdV1::from_digest_bytes([0xab; 32]);
    let entry = CatalogEntry {
        semantic_contract_id: wrong_id.clone(),
        manifest: synthetic_legacy_default_semantic_manifest(),
        rules_manifest: synthetic_legacy_default_rules_manifest(),
    };
    let catalog = RuntimeSemanticCatalog::from_entries(vec![entry]);

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: wrong_id.clone(),
    };
    let checkpoint = valid_v6_checkpoint(identity);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::SemanticContractDigestMismatch
    );
}

#[test]
fn turn_structure_task11_wrong_semantic_id_restore_is_nonmutating() {
    let wrong_id = SemanticContractIdV1::from_digest_bytes([0xab; 32]);
    let catalog = RuntimeSemanticCatalog::from_entries(vec![CatalogEntry {
        semantic_contract_id: wrong_id.clone(),
        manifest: synthetic_legacy_default_semantic_manifest(),
        rules_manifest: synthetic_legacy_default_rules_manifest(),
    }]);
    let controller = TrustedEnvironmentController::new(backend());
    let checkpoint = valid_v6_checkpoint(ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: wrong_id,
    });

    assert_controller_restore_rejection_is_nonmutating(
        "s1.catalog.wrong-semantic-id",
        &controller,
        checkpoint,
        catalog,
        |error| {
            matches!(
                error,
                ControllerError::CheckpointValidation(
                    CheckpointValidationError::SemanticContractDigestMismatch
                )
            )
        },
    );
}

#[test]
fn turn_structure_task11_wrong_closure_is_content_derived_and_not_exact_s1() {
    let (catalog, wrong_semantic_id, exact_semantic_id) = valid_wrong_closure_catalog();
    let production_catalog = RuntimeSemanticCatalog::production();
    let exact_entry = production_catalog.resolve(&exact_semantic_id).unwrap();
    let wrong_entry = catalog.resolve(&wrong_semantic_id).unwrap();
    let wrong_rules_id = wrong_entry.manifest.rules_contract_id.clone();
    assert_ne!(wrong_rules_id, exact_entry.manifest.rules_contract_id);
    assert_ne!(wrong_semantic_id, exact_semantic_id);
    assert!(catalog.resolve(&wrong_semantic_id).is_some());
    assert_eq!(
        calculate_rules_contract_id_v1(&wrong_entry.rules_manifest).unwrap(),
        wrong_rules_id
    );
    assert_eq!(
        calculate_semantic_contract_id_v1(&wrong_entry.manifest).unwrap(),
        wrong_semantic_id
    );
    assert!(matches!(
        admit_restore(
            &catalog,
            &s1_checkpoint(ExecutionIdentityV1 {
                program_kind: ExecutionProgramV1::MagicRules,
                semantic_contract_id: wrong_semantic_id.clone(),
            })
        ),
        Err(RestoreAdmissionError::SemanticContractUnsupported)
    ));
}

#[test]
fn turn_structure_task11_unsupported_restore_reaches_runtime_support_phase_and_is_nonmutating() {
    let (catalog, wrong_semantic_id, _) = valid_wrong_closure_catalog();
    let controller = TrustedEnvironmentController::new(backend());
    let checkpoint = s1_checkpoint(ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: wrong_semantic_id,
    });

    assert_controller_restore_rejection_is_nonmutating(
        "s1.unsupported-restore",
        &controller,
        checkpoint,
        catalog,
        |error| matches!(error, ControllerError::SemanticContractUnsupported),
    );
}

// === Phase 5: rules contract digest mismatch ===

#[test]
fn rules_contract_digest_mismatch_rejected() {
    // Build a valid ComprehensiveRules manifest + its derived IDs, then create
    // a catalog entry whose rules_manifest is the SyntheticLegacy manifest (which
    // recomputes to a DIFFERENT rules ID than the manifest's rules_contract_id).
    let cr_rules_manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "test-cr-2026-01-01".to_string(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/test".to_string(),
            version: "1.0.0".to_string(),
        }]),
    };
    let cr_rules_id = calculate_rules_contract_id_v1(&cr_rules_manifest).unwrap();
    let cr_manifest = SemanticContractManifestV1 {
        rules_contract_id: cr_rules_id,
        format_contract_id: None,
        content_contract_id: None,
    };
    let cr_semantic_id = calculate_semantic_contract_id_v1(&cr_manifest).unwrap();

    // Entry: semantic ID + manifest are consistent (phase 4 passes), but the
    // rules_manifest is the SyntheticLegacy one (phase 5 fails).
    let entry = CatalogEntry {
        semantic_contract_id: cr_semantic_id.clone(),
        manifest: cr_manifest,
        rules_manifest: synthetic_legacy_default_rules_manifest(),
    };
    let catalog = RuntimeSemanticCatalog::from_entries(vec![entry]);

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: cr_semantic_id,
    };
    let checkpoint = valid_v6_checkpoint(identity);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::RulesContractDigestMismatch
    );
}

// === Controller-level rejection nonmutation matrix ===
// (spec §12 observable invariant: rejected controller.restore() must not mutate
//  pre/post checkpoint or replay identity for ANY admission-rejection phase)

#[test]
fn controller_restore_rejects_corrupt_checkpoint_without_mutation() {
    let controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();

    let mut checkpoint = valid_v6_checkpoint(synthetic_identity());
    checkpoint.state_digest = FullStateDigestV5::from_digest_bytes([0xff; 32]);
    let result = controller.restore(checkpoint);
    assert!(matches!(
        result,
        Err(ControllerError::CheckpointValidation(
            CheckpointValidationError::StateDigest
        ))
    ));
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), before_replay);
}

#[test]
fn controller_restore_rejects_unknown_contract_without_mutation() {
    let controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: SemanticContractIdV1::from_digest_bytes([0u8; 32]),
    };
    let checkpoint = valid_v6_checkpoint(identity);
    let result = controller.restore(checkpoint);
    assert!(matches!(
        result,
        Err(ControllerError::CheckpointValidation(
            CheckpointValidationError::SemanticContractUnknown
        ))
    ));
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), before_replay);
}

#[test]
fn controller_restore_rejects_program_authority_mismatch_without_mutation() {
    let controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: synthetic_legacy_default_semantic_contract_id(),
    };
    let checkpoint = valid_v6_checkpoint(identity);
    let result = controller.restore(checkpoint);
    assert!(matches!(result, Err(ControllerError::ProgramAuthorityMismatch)));
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), before_replay);
}

#[test]
fn controller_restore_rejects_incompatible_state_without_mutation() {
    let controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();

    let state = synthetic_incompatible_state();
    let checkpoint = EnvironmentCheckpointV6::new(
        state,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        v6_codec(),
        synthetic_identity(),
    )
    .unwrap();
    let result = controller.restore(checkpoint);
    assert!(matches!(
        result,
        Err(ControllerError::ProgramStateIncompatible)
    ));
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), before_replay);
}

#[test]
fn controller_restore_rejects_semantic_contract_digest_mismatch_without_mutation() {
    let wrong_id = SemanticContractIdV1::from_digest_bytes([0xab; 32]);
    let entry = CatalogEntry {
        semantic_contract_id: wrong_id.clone(),
        manifest: synthetic_legacy_default_semantic_manifest(),
        rules_manifest: synthetic_legacy_default_rules_manifest(),
    };
    let catalog = RuntimeSemanticCatalog::from_entries(vec![entry]);

    let controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: wrong_id,
    };
    let checkpoint = valid_v6_checkpoint(identity);
    let result = controller.restore_with_catalog(checkpoint, catalog);
    assert!(matches!(
        result,
        Err(ControllerError::CheckpointValidation(
            CheckpointValidationError::SemanticContractDigestMismatch
        ))
    ));
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), before_replay);
}

#[test]
fn controller_restore_rejects_rules_contract_digest_mismatch_without_mutation() {
    let cr_rules_manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "test-cr-2026-01-01".to_string(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/test".to_string(),
            version: "1.0.0".to_string(),
        }]),
    };
    let cr_rules_id = calculate_rules_contract_id_v1(&cr_rules_manifest).unwrap();
    let cr_manifest = SemanticContractManifestV1 {
        rules_contract_id: cr_rules_id,
        format_contract_id: None,
        content_contract_id: None,
    };
    let cr_semantic_id = calculate_semantic_contract_id_v1(&cr_manifest).unwrap();

    let entry = CatalogEntry {
        semantic_contract_id: cr_semantic_id.clone(),
        manifest: cr_manifest,
        rules_manifest: synthetic_legacy_default_rules_manifest(),
    };
    let catalog = RuntimeSemanticCatalog::from_entries(vec![entry]);

    let controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: cr_semantic_id,
    };
    let checkpoint = valid_v6_checkpoint(identity);
    let result = controller.restore_with_catalog(checkpoint, catalog);
    assert!(matches!(
        result,
        Err(ControllerError::CheckpointValidation(
            CheckpointValidationError::RulesContractDigestMismatch
        ))
    ));
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), before_replay);
}

#[test]
fn controller_restore_rejects_unsupported_program_without_mutation() {
    let cr_rules_manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "test-cr-2026-01-01".to_string(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/test".to_string(),
            version: "1.0.0".to_string(),
        }]),
    };
    let cr_rules_id = calculate_rules_contract_id_v1(&cr_rules_manifest).unwrap();
    let cr_manifest = SemanticContractManifestV1 {
        rules_contract_id: cr_rules_id,
        format_contract_id: None,
        content_contract_id: None,
    };
    let cr_semantic_id = calculate_semantic_contract_id_v1(&cr_manifest).unwrap();

    let entry = CatalogEntry {
        semantic_contract_id: cr_semantic_id.clone(),
        manifest: cr_manifest,
        rules_manifest: cr_rules_manifest,
    };
    let catalog = RuntimeSemanticCatalog::from_entries(vec![entry]);

    let controller = TrustedEnvironmentController::new(backend());
    let before_checkpoint = controller.checkpoint().unwrap();
    let before_replay = controller.export_replay().unwrap();

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: cr_semantic_id,
    };
    let checkpoint = valid_v6_checkpoint(identity);
    let result = controller.restore_with_catalog(checkpoint, catalog);
    assert!(matches!(result, Err(ControllerError::SemanticContractUnsupported)));
    assert_eq!(controller.checkpoint().unwrap(), before_checkpoint);
    assert_eq!(controller.export_replay().unwrap(), before_replay);
}

// === Phase 6: program × authority mismatch ===

#[test]
fn program_authority_mismatch_rejected() {
    let catalog = RuntimeSemanticCatalog::production();
    // SyntheticLegacy has synthetic_legacy authority; MagicRules pairs only
    // with comprehensive_rules → mismatch.
    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: synthetic_legacy_default_semantic_contract_id(),
    };
    let checkpoint = valid_v6_checkpoint(identity);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::ProgramAuthorityMismatch
    );
}

// === Phase 7: runtime unsupported ===

#[test]
fn unsupported_program_rejected_as_semantic_contract_unsupported() {
    // Build a valid ComprehensiveRules contract + SyntheticLegacy catalog entry,
    // then create a checkpoint with MagicRules + the CR contract ID.
    // Phase 3: CR contract resolves (known meaning)
    // Phase 4: semantic manifest recomputes to CR semantic ID (matches)
    // Phase 5: rules manifest recomputes to CR rules ID (matches)
    // Phase 6: MagicRules × ComprehensiveRules is the frozen compatible pairing (passes)
    // Phase 7: MagicRules is NOT supported by this runtime (pre-S1) → SemanticContractUnsupported
    let cr_rules_manifest = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "test-cr-2026-01-01".to_string(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/test".to_string(),
            version: "1.0.0".to_string(),
        }]),
    };
    let cr_rules_id = calculate_rules_contract_id_v1(&cr_rules_manifest).unwrap();
    let cr_manifest = SemanticContractManifestV1 {
        rules_contract_id: cr_rules_id,
        format_contract_id: None,
        content_contract_id: None,
    };
    let cr_semantic_id = calculate_semantic_contract_id_v1(&cr_manifest).unwrap();

    let entry = CatalogEntry {
        semantic_contract_id: cr_semantic_id.clone(),
        manifest: cr_manifest,
        rules_manifest: cr_rules_manifest,
    };
    let catalog = RuntimeSemanticCatalog::from_entries(vec![entry]);

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: cr_semantic_id,
    };
    let checkpoint = valid_v6_checkpoint(identity);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::SemanticContractUnsupported
    );
}

// === Phase 8: program × EngineState incompatibility ===

#[test]
fn program_state_incompatible_rejected() {
    let catalog = RuntimeSemanticCatalog::production();
    let state = synthetic_incompatible_state();
    let codec = v6_codec();
    let identity = synthetic_identity();
    let checkpoint = EnvironmentCheckpointV6::new(
        state,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        codec,
        identity,
    )
    .unwrap();
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::ProgramStateIncompatible
    );
}

// === Positive: valid SyntheticRulesCompat + SyntheticLegacy admits ===

#[test]
fn synthetic_legacy_admits_under_synthetic_program() {
    let catalog = RuntimeSemanticCatalog::production();
    let checkpoint = valid_v6_checkpoint(synthetic_identity());
    let result = admit_restore(&catalog, &checkpoint);
    assert!(result.is_ok(), "valid synthetic restore must be admitted");
}

// === Admission phase ordering ===

#[test]
fn earlier_phase_defects_are_not_reported_as_later_failures() {
    // An unknown semantic contract (phase 3) must NOT be reported as a
    // program-authority mismatch (phase 6) or unsupported (phase 7).
    let catalog = RuntimeSemanticCatalog::production();
    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: SemanticContractIdV1::from_digest_bytes([0u8; 32]),
    };
    let checkpoint = valid_v6_checkpoint(identity);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::SemanticContractUnknown
    );
}

// === Phase ordering: digest mismatch precedes authority check ===

#[test]
fn digest_mismatch_precedes_authority_mismatch() {
    // A catalog entry with a mismatched semantic ID (phase 4) must fail at
    // phase 4, NOT reported as a phase-6 ProgramAuthorityMismatch.
    let wrong_id = SemanticContractIdV1::from_digest_bytes([0xab; 32]);
    let entry = CatalogEntry {
        semantic_contract_id: wrong_id.clone(),
        manifest: synthetic_legacy_default_semantic_manifest(),
        rules_manifest: synthetic_legacy_default_rules_manifest(),
    };
    let catalog = RuntimeSemanticCatalog::from_entries(vec![entry]);

    let identity = ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::MagicRules,
        semantic_contract_id: wrong_id,
    };
    let checkpoint = valid_v6_checkpoint(identity);
    let result = admit_restore(&catalog, &checkpoint);
    assert_eq!(
        result.unwrap_err(),
        RestoreAdmissionError::SemanticContractDigestMismatch
    );
}
