use super::*;
use mtgml_card_ir::{
    AbilityIdentityV1, BaseCharacteristicsV1, BasicLandProfileV1, BasicLandSubtypeV1,
    CardDefinitionEnvelopeV1, CardSemanticBindingV1, CardSemanticProfileId,
    ContentContractManifestV1, DefinitionProvenanceRecordV1, FaceDefinitionV1, FaceKey,
    ProvenanceCatalogV1, SourceProvenanceV1, TypeLineV1, VerifiedContentCatalogV1,
    BASIC_LAND_PROFILE_ID_V1,
};
use mtgml_model::{
    CapabilityRequirementV1, CardDefinitionId, ExecutionIdentityV1, ExecutionProgramV1,
    RulesAuthorityV1, RulesContractManifestV1, SemanticContractManifestV1,
};
use mtgml_observation::MagicSharedExecutionObservationV1;
use mtgml_state::{
    AbilityAuthorityStateV1, AttachmentStateV1, CardRulesAuthoritativeStateV1, CounterKindV1,
    CounterStateV1, EngineStatePartsV3, ExecutionStateV4, FaceStateV1, ManaPoolV1, ManaStateV1,
    PlayerTurnHistoryV1, TurnHistoryStateV1,
};
use std::collections::BTreeMap;

fn catalog_for(definition_ids: &[u64]) -> VerifiedContentCatalogV1 {
    let definitions = definition_ids
        .iter()
        .map(|id| {
            let (name, type_line, subtype) = match *id {
                1 => ("Mountain", "Land", Some("Mountain")),
                2 => ("Plains", "Land", Some("Plains")),
                _ => ("Projection Test Creature", "Creature", None),
            };
            CardDefinitionEnvelopeV1 {
                envelope_version: "card-definition-envelope.v1".to_owned(),
                card_definition_id: CardDefinitionId(*id),
                faces: vec![FaceDefinitionV1 {
                    face_key: FaceKey(0),
                    base_characteristics: BaseCharacteristicsV1 {
                        name: name.into(),
                        mana_cost: if subtype.is_none() {
                            Some(vec![mtgml_card_ir::PrintedManaSymbolV1::Generic(1)])
                        } else {
                            None
                        },
                        color_indicator: vec![],
                        type_line: TypeLineV1 {
                            supertypes: if subtype.is_some() {
                                vec!["Basic".into()]
                            } else {
                                vec![]
                            },
                            card_types: vec![type_line.into()],
                            subtypes: subtype.into_iter().map(str::to_owned).collect(),
                        },
                        power_toughness: subtype.is_none().then_some((2, 2)),
                        loyalty: None,
                        defense: None,
                    },
                }],
                ability_identities: if subtype.is_some() {
                    vec![AbilityIdentityV1 {
                        ability_key: mtgml_card_ir::AbilityKey(0),
                        face_key: FaceKey(0),
                    }]
                } else {
                    vec![]
                },
                semantic_binding: match subtype {
                    Some("Mountain") => CardSemanticBindingV1::ProfiledV1 {
                        profile_id: CardSemanticProfileId::parse(BASIC_LAND_PROFILE_ID_V1).unwrap(),
                        body: BasicLandProfileV1 {
                            subtype: BasicLandSubtypeV1::Mountain,
                        },
                    },
                    Some("Plains") => CardSemanticBindingV1::ProfiledV1 {
                        profile_id: CardSemanticProfileId::parse(BASIC_LAND_PROFILE_ID_V1).unwrap(),
                        body: BasicLandProfileV1 {
                            subtype: BasicLandSubtypeV1::Plains,
                        },
                    },
                    _ => CardSemanticBindingV1::UnprofiledV1,
                },
                definition_references: vec![],
                explicit_additional_requirements: vec![],
            }
        })
        .collect();
    let manifest = ContentContractManifestV1 {
        schema_version: "content-contract-manifest.v1".into(),
        definitions,
    };
    let bytes = mtgml_card_ir::encode_content_manifest_v1(&manifest).unwrap();
    let id = mtgml_persistence::content_contract_digest::calculate_content_contract_id_v1(&bytes)
        .unwrap();
    let provenance = ProvenanceCatalogV1 {
        schema_version: "definition-provenance-catalog.v1".into(),
        records: definition_ids
            .iter()
            .map(|definition| DefinitionProvenanceRecordV1 {
                content_contract_id: id.clone(),
                card_definition_id: CardDefinitionId(*definition),
                source_provenance: SourceProvenanceV1 {
                    source_snapshot_id: "snapshot/test-1".into(),
                    source_record_id: format!("record/{definition}"),
                    source_record_codec_id: "test-fixture.v1".into(),
                    source_record_digest: [0x5a; 32],
                },
            })
            .collect(),
    };
    VerifiedContentCatalogV1::build(&bytes, &id, provenance).unwrap()
}

fn execution_authority(
    catalog: &VerifiedContentCatalogV1,
) -> (
    ExecutionIdentityV1,
    SemanticContractManifestV1,
    RulesContractManifestV1,
) {
    let rules = RulesContractManifestV1 {
        rules_authority: RulesAuthorityV1::ComprehensiveRules {
            snapshot_id: "rules-fixture-snapshot".into(),
        },
        capability_closure: Some(vec![CapabilityRequirementV1 {
            key: "rules/land-play".into(),
            version: "0.1.0".into(),
        }]),
    };
    let rules_id =
        mtgml_persistence::semantic_contract_digest::calculate_rules_contract_id_v1(&rules)
            .unwrap();
    let semantic = SemanticContractManifestV1 {
        rules_contract_id: rules_id,
        format_contract_id: None,
        content_contract_id: Some(catalog.content_contract_id().clone()),
    };
    let semantic_id =
        mtgml_persistence::semantic_contract_digest::calculate_semantic_contract_id_v1(&semantic)
            .unwrap();
    (
        ExecutionIdentityV1 {
            program_kind: ExecutionProgramV1::MagicRules,
            semantic_contract_id: semantic_id,
        },
        semantic,
        rules,
    )
}

pub(crate) fn basic_land_state(root_seed: mtgml_random::RootSeed256) -> EngineStatePartsV3 {
    let mut state =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed,
            setup: mtgml_state::SyntheticV4Setup::synthetic_compatibility(),
        })
        .unwrap();
    for player in [PlayerId(1), PlayerId(2)] {
        let identity = state
            .perspective_identities
            .players
            .get_mut(&player)
            .unwrap();
        for (object, location) in &state.zones.locations {
            if location.zone == mtgml_model::ZoneKind::Battlefield
                && !identity.object_to_opaque.contains_key(object)
            {
                let opaque = identity.next_opaque_object_id;
                identity.next_opaque_object_id.0 += 1;
                identity.object_to_opaque.insert(*object, opaque);
                identity.opaque_to_object.insert(opaque, *object);
            }
        }
    }
    let mut mana = ManaStateV1::default();
    let mut history = BTreeMap::new();
    for player in [PlayerId(1), PlayerId(2)] {
        let mut pool = ManaPoolV1::default();
        pool.unrestricted[3] = u32::from(player == PlayerId(1));
        mana.pools.insert(player, pool);
        history.insert(player, PlayerTurnHistoryV1::default());
    }
    let mut faces = FaceStateV1::default();
    for object in state.zones.objects.keys() {
        faces.faces.insert(*object, 0);
    }
    let mut counters = CounterStateV1::default();
    if let Some((object, _)) = state
        .zones
        .locations
        .iter()
        .find(|(_, location)| location.zone == mtgml_model::ZoneKind::Battlefield)
    {
        counters.counters.insert(
            *object,
            BTreeMap::from([(CounterKindV1::PlusOnePlusOne, 2)]),
        );
    }
    let mut parts = state.parts();
    parts.execution = Default::default();
    EngineStatePartsV3::new(
        parts,
        ExecutionStateV4::default(),
        CardRulesAuthoritativeStateV1 {
            mana,
            turn_history: TurnHistoryStateV1 {
                turn_number: state.core.turn_number,
                players: history,
                ..TurnHistoryStateV1::default()
            },
            counters,
            attachments: AttachmentStateV1::default(),
            faces,
            abilities: AbilityAuthorityStateV1::default(),
        },
    )
    .unwrap()
}

#[test]
fn magic_basic_land_projection_uses_opaque_public_ids_and_verified_content_faces() {
    let state = basic_land_state(seed());
    state.validate().unwrap();
    let catalog = catalog_for(&[1, 2]);
    let (identity, semantic, rules) = execution_authority(&catalog);
    let projected = crate::player_projection::project_magic_basic_land_observation(
        &state,
        PlayerId(1),
        &identity,
        &semantic,
        &rules,
        &catalog,
    )
    .unwrap();
    assert_eq!(
        projected.schema_version,
        mtgml_observation::MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1
    );
    let payload = mtgml_wire::encode_canonical(&projected).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    assert_eq!(value["mana_pools"][0]["unrestricted"][3], 1);
    assert_eq!(value["counters"][0]["object"], "1");
    assert_eq!(value["faces"][0]["face"], "front");
    let encoded = String::from_utf8(payload).unwrap();
    for forbidden in [
        "game_object_id",
        "physical_card_id",
        "face_key",
        "card_definition_id",
    ] {
        assert!(!encoded.contains(forbidden), "leaked {forbidden}");
    }
}

#[test]
fn g0g_information_projection_uses_shared_stack_codec_without_global_revision() {
    let successor = basic_land_state(seed());
    let catalog = catalog_for(&[1, 2]);
    let (identity, semantic, rules) = execution_authority(&catalog);
    let information = crate::player_projection::project_successor_information_state_v3(
        &successor,
        PlayerId(1),
        &identity,
        &semantic,
        &rules,
        &catalog,
    )
    .unwrap();
    let wire = mtgml_wire::encode_canonical(&information).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&wire).unwrap();
    assert_eq!(
        value["current_observation"]["payload_codec"],
        mtgml_observation::MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1
    );
    assert!(value.get("state_revision").is_none());
    assert!(value["current_observation"].get("state_revision").is_none());
    let payload = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        value["current_observation"]["payload_base64"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let observation: MagicSharedExecutionObservationV1 =
        mtgml_wire::decode_canonical(&payload).unwrap();
    observation.validate().unwrap();
    assert!(observation.stack.is_empty());
    assert!(observation.temporary_effects.is_empty());

    let mut later_global_revision = successor.clone();
    later_global_revision.predecessor_v5.revision.0 += 11;
    let later_information = crate::player_projection::project_successor_information_state_v3(
        &later_global_revision,
        PlayerId(1),
        &identity,
        &semantic,
        &rules,
        &catalog,
    )
    .unwrap();
    assert_eq!(
        mtgml_wire::encode_canonical(&later_information).unwrap(),
        wire,
        "global revisions must not change a player's V3 information bytes"
    );
}

#[test]
fn g0g_projection_adds_shared_views_and_hides_revision_and_allocator_history() {
    let mut state = basic_land_state(seed());
    let target_object = state
        .predecessor_v5
        .zones
        .locations
        .iter()
        .find_map(|(object, location)| {
            (location.zone == mtgml_model::ZoneKind::Battlefield).then_some(*object)
        })
        .unwrap();

    let stack_card = CardDefinitionId(50);
    let stack_object = mtgml_model::GameObjectId(50);
    let stack_id = mtgml_model::StackObjectId(1);
    state.predecessor_v5.zones.objects.insert(
        stack_object,
        mtgml_state::GameObject {
            id: stack_object,
            physical_card: Some(mtgml_model::PhysicalCardId(50)),
            card_definition: stack_card,
            owner: PlayerId(1),
            controller: PlayerId(1),
            tapped: false,
            face_down: false,
        },
    );
    state.predecessor_v5.zones.locations.insert(
        stack_object,
        mtgml_state::ZoneLocation {
            zone: mtgml_model::ZoneKind::Stack,
            player: None,
            position: mtgml_state::ZonePosition::Unordered,
            visibility: mtgml_state::VisibilityPartition::Public,
            partition: None,
        },
    );
    state.predecessor_v5.allocators.next_object_id = mtgml_model::GameObjectId(51);
    state.predecessor_v5.allocators.next_stack_object_id = mtgml_model::StackObjectId(2);
    state.predecessor_v5.allocators.next_effect_id = mtgml_model::EffectInstanceId(2);
    state.card_rules_state.faces.faces.insert(stack_object, 0);
    state.predecessor_v5.zones.stack_order.push(stack_id);
    state.predecessor_v5.zones.stack_records.insert(
        stack_id,
        mtgml_state::StackRecord {
            id: stack_id,
            controller: PlayerId(1),
            source_object: None,
            source_ability: None,
            payload: Some(mtgml_state::StackItemPayload::Spell {
                stack_card_object: stack_object,
                card_definition_id: stack_card,
                face_key: FaceKey(0),
                semantic_profile_id: CardSemanticProfileId::parse("test/projection@1.0.0").unwrap(),
                modes: vec![mtgml_state::ModeBinding {
                    mode_slot: 0,
                    selected_mode: 2,
                }],
                targets: vec![mtgml_state::TargetBinding {
                    target_slot: 0,
                    target: mtgml_state::TargetRef::Object(target_object),
                }],
                cost_facts: mtgml_state::CostFacts::default(),
            }),
        },
    );
    for player in [PlayerId(1), PlayerId(2)] {
        let perspective = state
            .predecessor_v5
            .perspective_identities
            .players
            .get_mut(&player)
            .unwrap();
        let opaque = perspective.next_opaque_object_id;
        perspective.next_opaque_object_id.0 += 1;
        perspective.object_to_opaque.insert(stack_object, opaque);
        perspective.opaque_to_object.insert(opaque, stack_object);
        state
            .predecessor_v5
            .knowledge
            .players
            .get_mut(&player)
            .unwrap()
            .active
            .insert(
                opaque,
                mtgml_state::KnowledgeRecordV2 {
                    opaque_object: opaque,
                    physical_card: Some(mtgml_model::PhysicalCardId(50)),
                    card_definition: Some(stack_card),
                    known_location: Some(mtgml_state::KnownLocationFactV2 {
                        location: state.predecessor_v5.zones.locations[&stack_object].clone(),
                        provenance: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                    }),
                    historical_locations: vec![],
                    acquisition: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                },
            );
    }
    let hidden_card = mtgml_model::GameObjectId(51);
    state.predecessor_v5.zones.objects.insert(
        hidden_card,
        mtgml_state::GameObject {
            id: hidden_card,
            physical_card: Some(mtgml_model::PhysicalCardId(51)),
            card_definition: CardDefinitionId(1),
            owner: PlayerId(2),
            controller: PlayerId(2),
            tapped: false,
            face_down: true,
        },
    );
    let hidden_location = mtgml_state::ZoneLocation {
        zone: mtgml_model::ZoneKind::Library,
        player: Some(PlayerId(2)),
        position: mtgml_state::ZonePosition::Top { offset: 1 },
        visibility: mtgml_state::VisibilityPartition::FaceDown,
        partition: None,
    };
    state
        .predecessor_v5
        .zones
        .locations
        .insert(hidden_card, hidden_location.clone());
    let hidden_zone_key = hidden_location.key();
    state
        .predecessor_v5
        .zones
        .ordered_zones
        .get_mut(&hidden_zone_key)
        .unwrap()
        .push(hidden_card);
    state.predecessor_v5.allocators.next_object_id = mtgml_model::GameObjectId(52);
    state.card_rules_state.faces.faces.insert(hidden_card, 0);
    let owner_identity = state
        .predecessor_v5
        .perspective_identities
        .players
        .get_mut(&PlayerId(2))
        .unwrap();
    let hidden_opaque = owner_identity.next_opaque_object_id;
    owner_identity.next_opaque_object_id.0 += 1;
    owner_identity
        .object_to_opaque
        .insert(hidden_card, hidden_opaque);
    owner_identity
        .opaque_to_object
        .insert(hidden_opaque, hidden_card);
    state
        .predecessor_v5
        .knowledge
        .players
        .get_mut(&PlayerId(2))
        .unwrap()
        .active
        .insert(
            hidden_opaque,
            mtgml_state::KnowledgeRecordV2 {
                opaque_object: hidden_opaque,
                physical_card: Some(mtgml_model::PhysicalCardId(51)),
                card_definition: Some(CardDefinitionId(1)),
                known_location: Some(mtgml_state::KnownLocationFactV2 {
                    location: hidden_location,
                    provenance: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
                }),
                historical_locations: vec![],
                acquisition: mtgml_state::KnowledgeAcquisitionReason::InitialConfiguration,
            },
        );
    let before_effect = state.clone();
    state.execution_v4.effects.insert(
        mtgml_model::EffectInstanceId(1),
        mtgml_state::TemporaryEffectRecord {
            id: mtgml_model::EffectInstanceId(1),
            affected_objects: vec![target_object],
            operation: mtgml_state::TemporaryOperation::PowerToughnessDelta {
                power: 1,
                toughness: -1,
            },
            expiry: mtgml_state::EffectExpiry::UntilEndOfTurn {
                turn_number: state.predecessor_v5.core.turn_number,
            },
            timestamp: None,
        },
    );
    state.validate().unwrap();

    let catalog = catalog_for(&[1, 2, 50]);
    let (identity, semantic, rules) = execution_authority(&catalog);
    let information = crate::player_projection::project_successor_information_state_v3(
        &state,
        PlayerId(1),
        &identity,
        &semantic,
        &rules,
        &catalog,
    )
    .unwrap();
    let payload = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &information.current_observation.payload_base64,
    )
    .unwrap();
    let observation: MagicSharedExecutionObservationV1 =
        mtgml_wire::decode_canonical(&payload).unwrap();
    observation.validate().unwrap();
    assert_eq!(observation.stack.len(), 1);
    assert!(matches!(
        &observation.stack[0],
        mtgml_observation::PublicStackItemV1::Spell {
            card_object,
            modes,
            targets,
            ..
        } if card_object.0 > 0
            && modes == &[mtgml_observation::PublicModeV1 {
                mode_slot: 0,
                selected_mode: 2,
            }]
            && matches!(targets.as_slice(), [mtgml_decision::SafeTargetDescriptorV1::Object { .. }])
    ));
    assert_eq!(observation.temporary_effects.len(), 1);
    let encoded = mtgml_wire::encode_canonical(&observation).unwrap();
    for forbidden in [
        "game_object_id",
        "physical_card_id",
        "stack_object_id",
        "effect_instance_id",
        "semantic_profile_id",
    ] {
        assert!(!String::from_utf8_lossy(&encoded).contains(forbidden));
    }

    let after_revision = mtgml_model::StateRevision(before_effect.predecessor_v5.revision.0 + 1);
    state.predecessor_v5.revision = after_revision;
    state.predecessor_v5.allocators.next_rule_event_id = mtgml_model::RuleEventId(4);
    let effect = state.execution_v4.effects[&mtgml_model::EffectInstanceId(1)].clone();
    let mut events = vec![mtgml_rules::AuthoritativeRuleEventV3 {
        event_id: mtgml_model::RuleEventId(1),
        state_revision: after_revision,
        event: mtgml_rules::AuthoritativeRuleEventKindV3::TemporaryEffectCreated { effect },
    }];
    let mut after_engine: mtgml_state::EngineState = state.predecessor_v5.clone().into();
    for event_id in [2, 3] {
        let perspective = PlayerId(event_id - 1);
        let sequence =
            before_effect.predecessor_v5.knowledge.players[&perspective].next_visible_sequence;
        let lifecycle = mtgml_state::PerspectiveLifecycleAuditV1 {
            perspective,
            sequence,
            mutation: mtgml_state::PerspectiveLifecycleMutationV1::default(),
        };
        mtgml_state::apply_perspective_lifecycle(&mut after_engine, &lifecycle).unwrap();
        events.push(mtgml_rules::AuthoritativeRuleEventV3 {
            event_id: mtgml_model::RuleEventId(event_id),
            state_revision: after_revision,
            event: mtgml_rules::AuthoritativeRuleEventKindV3::PerspectiveObservationOccurrence {
                lifecycle: Box::new(lifecycle),
                source_event_id: mtgml_model::RuleEventId(1),
            },
        });
    }
    state.predecessor_v5 = after_engine.parts();
    let (execution_identity, semantic_manifest, rules_manifest) = execution_authority(&catalog);
    let running = mtgml_model::EpisodeStatus::Running;
    let project_steps = |before: &mtgml_state::EngineStatePartsV3,
                         after: &mtgml_state::EngineStatePartsV3,
                         events: &[mtgml_rules::AuthoritativeRuleEventV3]| {
        crate::successor_projection::project_successor_player_steps_v4(
            crate::successor_projection::SuccessorTransitionV4Projection {
                before,
                after,
                before_status: &running,
                events,
                delta: None,
                accepted: true,
                status: &running,
                next_request: None,
                actor: PlayerId(1),
                rejected_code: mtgml_observation::PlayerSubmissionCodeV1::InvalidAnswer,
            },
            crate::successor_projection::SuccessorProjectionAuthority {
                execution_identity: &execution_identity,
                semantic_manifest: &semantic_manifest,
                rules_manifest: &rules_manifest,
                catalog: &catalog,
                basic_land_admission: None,
            },
        )
    };
    let steps = project_steps(&before_effect, &state, &events).unwrap();
    assert_eq!(steps[&PlayerId(1)].observed_events.len(), 1);
    assert_eq!(steps[&PlayerId(2)].observed_events.len(), 1);

    let mut occurrence = events[1].clone();
    let mut source = events[0].clone();
    let mtgml_rules::AuthoritativeRuleEventKindV3::PerspectiveObservationOccurrence {
        source_event_id,
        ..
    } = &mut occurrence.event
    else {
        unreachable!()
    };
    *source_event_id = mtgml_model::RuleEventId(2);
    occurrence.event_id = mtgml_model::RuleEventId(1);
    source.event_id = mtgml_model::RuleEventId(2);
    let mut future_reference_events = vec![occurrence, source, events[2].clone()];
    if let mtgml_rules::AuthoritativeRuleEventKindV3::PerspectiveObservationOccurrence {
        source_event_id,
        ..
    } = &mut future_reference_events[2].event
    {
        *source_event_id = mtgml_model::RuleEventId(2);
    }
    assert!(project_steps(&before_effect, &state, &future_reference_events).is_err());

    let step_bytes = [PlayerId(1), PlayerId(2)]
        .map(|player| mtgml_wire::encode_canonical(&steps[&player]).unwrap());
    for bytes in &step_bytes {
        let json = String::from_utf8_lossy(bytes);
        for forbidden in [
            "state_revision",
            "game_object_id",
            "stack_object_id",
            "effect_instance_id",
            "source_event_id",
        ] {
            assert!(!json.contains(forbidden), "leaked {forbidden}");
        }
    }

    let mut hidden_revision_before = before_effect.clone();
    hidden_revision_before.predecessor_v5.revision.0 += 17;
    hidden_revision_before
        .predecessor_v5
        .allocators
        .next_decision_id
        .0 += 7;
    hidden_revision_before
        .predecessor_v5
        .allocators
        .next_continuation_id
        .0 += 13;
    let mut hidden_revision_after = state.clone();
    hidden_revision_after.predecessor_v5.revision.0 += 17;
    hidden_revision_after
        .predecessor_v5
        .allocators
        .next_decision_id
        .0 += 7;
    hidden_revision_after
        .predecessor_v5
        .allocators
        .next_continuation_id
        .0 += 13;
    let hidden_revision_events = events
        .iter()
        .map(|event| {
            let mut event = event.clone();
            event.state_revision.0 += 17;
            event
        })
        .collect::<Vec<_>>();
    let hidden_revision_steps = project_steps(
        &hidden_revision_before,
        &hidden_revision_after,
        &hidden_revision_events,
    )
    .unwrap();
    assert_eq!(
        [PlayerId(1), PlayerId(2)]
            .map(|player| mtgml_wire::encode_canonical(&hidden_revision_steps[&player]).unwrap()),
        step_bytes,
        "global revision and allocator-history differences must not affect player V4 products"
    );

    let reidentify = |state: &mut mtgml_state::EngineStatePartsV3| {
        let mut record = state
            .predecessor_v5
            .zones
            .stack_records
            .remove(&stack_id)
            .unwrap();
        record.id = mtgml_model::StackObjectId(77);
        state
            .predecessor_v5
            .zones
            .stack_records
            .insert(record.id, record);
        for item in &mut state.predecessor_v5.zones.stack_order {
            if *item == stack_id {
                *item = mtgml_model::StackObjectId(77);
            }
        }
        state.predecessor_v5.allocators.next_stack_object_id = mtgml_model::StackObjectId(78);
        state.predecessor_v5.allocators.next_effect_id = mtgml_model::EffectInstanceId(91);
        if let Some(mut effect) = state
            .execution_v4
            .effects
            .remove(&mtgml_model::EffectInstanceId(1))
        {
            effect.id = mtgml_model::EffectInstanceId(90);
            state.execution_v4.effects.insert(effect.id, effect);
        }
    };
    let mut reidentified_before = before_effect.clone();
    let mut reidentified_after = state.clone();
    reidentify(&mut reidentified_before);
    reidentify(&mut reidentified_after);
    let mut reidentified_events = events.clone();
    if let mtgml_rules::AuthoritativeRuleEventKindV3::TemporaryEffectCreated { effect } =
        &mut reidentified_events[0].event
    {
        effect.id = mtgml_model::EffectInstanceId(90);
    }
    let reidentified_steps = project_steps(
        &reidentified_before,
        &reidentified_after,
        &reidentified_events,
    )
    .unwrap();
    assert_eq!(
        [PlayerId(1), PlayerId(2)]
            .map(|player| { mtgml_wire::encode_canonical(&reidentified_steps[&player]).unwrap() }),
        step_bytes,
        "trusted stack/effect identities must not affect player V4 products"
    );

    let reorder_hidden_library =
        |state: &mut mtgml_state::EngineStatePartsV3, order: [mtgml_model::GameObjectId; 2]| {
            let key = state.predecessor_v5.zones.locations[&order[0]].key();
            state
                .predecessor_v5
                .zones
                .ordered_zones
                .insert(key, order.to_vec());
            for (offset, object) in order.into_iter().enumerate() {
                let position = mtgml_state::ZonePosition::Top {
                    offset: u32::try_from(offset).unwrap(),
                };
                state
                    .predecessor_v5
                    .zones
                    .locations
                    .get_mut(&object)
                    .unwrap()
                    .position = position;
                let opaque = state.predecessor_v5.perspective_identities.players[&PlayerId(2)]
                    .object_to_opaque[&object];
                state
                    .predecessor_v5
                    .knowledge
                    .players
                    .get_mut(&PlayerId(2))
                    .unwrap()
                    .active
                    .get_mut(&opaque)
                    .unwrap()
                    .known_location
                    .as_mut()
                    .unwrap()
                    .location
                    .position = position;
            }
        };
    let reversed_library = [hidden_card, mtgml_model::GameObjectId(2)];
    let mut reordered_before = before_effect.clone();
    let mut reordered_after = state.clone();
    reorder_hidden_library(&mut reordered_before, reversed_library);
    reorder_hidden_library(&mut reordered_after, reversed_library);
    let reordered_steps = project_steps(&reordered_before, &reordered_after, &events).unwrap();
    assert_eq!(
        mtgml_wire::encode_canonical(&reordered_steps[&PlayerId(1)]).unwrap(),
        step_bytes[0],
        "an opponent's hidden library order must not affect this player's V4 product"
    );
    assert_eq!(
        mtgml_wire::encode_canonical(&reordered_steps[&PlayerId(2)]).unwrap(),
        step_bytes[1],
        "hidden library order must not appear in either player's V4 product"
    );

    let before_remove = state.clone();
    let mut after_remove = state.clone();
    let removal_revision = mtgml_model::StateRevision(before_remove.predecessor_v5.revision.0 + 1);
    after_remove.predecessor_v5.revision = removal_revision;
    after_remove.predecessor_v5.allocators.next_rule_event_id = mtgml_model::RuleEventId(7);
    after_remove
        .predecessor_v5
        .zones
        .stack_records
        .remove(&stack_id);
    after_remove
        .predecessor_v5
        .zones
        .stack_order
        .retain(|candidate| *candidate != stack_id);
    after_remove
        .predecessor_v5
        .zones
        .locations
        .get_mut(&stack_object)
        .unwrap()
        .zone = mtgml_model::ZoneKind::Graveyard;
    after_remove
        .predecessor_v5
        .zones
        .locations
        .get_mut(&stack_object)
        .unwrap()
        .player = Some(PlayerId(1));
    let removed_payload = state.predecessor_v5.zones.stack_records[&stack_id]
        .payload
        .as_ref()
        .unwrap()
        .clone();
    let mut removal_events = vec![mtgml_rules::AuthoritativeRuleEventV3 {
        event_id: mtgml_model::RuleEventId(4),
        state_revision: removal_revision,
        event: mtgml_rules::AuthoritativeRuleEventKindV3::StackItemRemoved {
            stack_object: stack_id,
            payload: removed_payload,
            result: mtgml_state::StackItemEndKindV1::Resolved,
        },
    }];
    let mut removal_engine: mtgml_state::EngineState = after_remove.predecessor_v5.clone().into();
    for event_id in [5, 6] {
        let perspective = PlayerId(event_id - 4);
        let sequence =
            before_remove.predecessor_v5.knowledge.players[&perspective].next_visible_sequence;
        let opaque = before_remove.predecessor_v5.perspective_identities.players[&perspective]
            .object_to_opaque[&stack_object];
        let destination = after_remove.predecessor_v5.zones.locations[&stack_object].clone();
        let lifecycle = mtgml_state::PerspectiveLifecycleAuditV1 {
            perspective,
            sequence,
            mutation: mtgml_state::PerspectiveLifecycleMutationV1 {
                identity: mtgml_state::IdentityMutationV1::None,
                knowledge: Some(mtgml_state::KnowledgeMutationV1::UpdateLocation {
                    opaque,
                    fact: mtgml_state::KnownLocationFactV2 {
                        location: destination,
                        provenance: mtgml_state::KnowledgeAcquisitionReason::Observed {
                            channel: mtgml_state::KnowledgeHistoryChannel::Public,
                            sequence,
                            cause: mtgml_state::KnowledgeAcquisitionCause::PublicEvent,
                        },
                    },
                }),
            },
        };
        mtgml_state::apply_perspective_lifecycle(&mut removal_engine, &lifecycle).unwrap();
        removal_events.push(mtgml_rules::AuthoritativeRuleEventV3 {
            event_id: mtgml_model::RuleEventId(event_id),
            state_revision: removal_revision,
            event: mtgml_rules::AuthoritativeRuleEventKindV3::PerspectiveObservationOccurrence {
                lifecycle: Box::new(lifecycle),
                source_event_id: mtgml_model::RuleEventId(4),
            },
        });
    }
    after_remove.predecessor_v5 = removal_engine.parts();
    after_remove.validate().unwrap();
    let removed_steps = project_steps(&before_remove, &after_remove, &removal_events).unwrap();
    for player in [PlayerId(1), PlayerId(2)] {
        assert!(matches!(
            removed_steps[&player].observed_events[0].event,
            mtgml_observation::ObservedEventKindV4::StackItemRemoved {
                stack_position_from_top: 0,
                cause: mtgml_observation::StackItemRemovalCauseV1::Resolved,
                ..
            }
        ));
    }
    let mut missing_audience = removal_events.clone();
    missing_audience.pop();
    assert!(project_steps(&before_remove, &after_remove, &missing_audience).is_err());
}

#[test]
fn verified_catalog_rejects_unknown_face_key_before_observation_projection() {
    let mut parts = basic_land_state(seed());
    let object = *parts.card_rules_state.faces.faces.keys().next().unwrap();
    parts.card_rules_state.faces.faces.insert(object, 99);
    let catalog = catalog_for(&[1, 2]);
    let (identity, semantic, rules) = execution_authority(&catalog);
    assert_eq!(
        crate::player_projection::project_magic_basic_land_observation(
            &parts,
            PlayerId(1),
            &identity,
            &semantic,
            &rules,
            &catalog,
        ),
        Err(PlayerEndpointError::ServiceUnavailable)
    );
}

#[test]
fn verified_catalog_rejects_ability_key_missing_from_the_selected_face() {
    let mut parts = basic_land_state(seed());
    let source = *parts.predecessor_v5.zones.objects.keys().next().unwrap();
    parts.predecessor_v5.allocators.next_ability_id = mtgml_model::AbilityInstanceId(2);
    parts.card_rules_state.abilities.by_instance.insert(
        mtgml_model::AbilityInstanceId(1),
        mtgml_state::AbilityAuthorityV1 {
            source,
            ability_key: 77,
        },
    );
    let catalog = catalog_for(&[1, 2]);
    let (identity, semantic, rules) = execution_authority(&catalog);
    assert_eq!(
        crate::player_projection::project_magic_basic_land_observation(
            &parts,
            PlayerId(1),
            &identity,
            &semantic,
            &rules,
            &catalog,
        ),
        Err(PlayerEndpointError::ServiceUnavailable)
    );
}

#[test]
fn hidden_rng_change_preserves_basic_land_observation_bytes_and_digest() {
    let a = basic_land_state(seed());
    let other_seed = mtgml_random::RootSeed256::from_lower_hex(&"99".repeat(32)).unwrap();
    let b = basic_land_state(other_seed);
    let catalog = catalog_for(&[1, 2]);
    let (identity, semantic, rules) = execution_authority(&catalog);
    let basic = |state: &EngineStatePartsV3| {
        let value = crate::player_projection::project_magic_basic_land_observation(
            state,
            PlayerId(1),
            &identity,
            &semantic,
            &rules,
            &catalog,
        )
        .unwrap();
        mtgml_wire::encode_canonical(&value).unwrap()
    };
    assert_eq!(basic(&a), basic(&b));
    let information = |state: &EngineStatePartsV3| {
        crate::player_projection::project_successor_information_state_v3(
            state,
            PlayerId(1),
            &identity,
            &semantic,
            &rules,
            &catalog,
        )
        .unwrap()
    };
    let (left_information, right_information) = (information(&a), information(&b));
    assert_eq!(left_information.digest, right_information.digest);
    assert_eq!(
        mtgml_wire::encode_canonical(&left_information).unwrap(),
        mtgml_wire::encode_canonical(&right_information).unwrap()
    );
}

#[test]
fn trusted_game_object_renaming_preserves_public_observation_and_information_bytes() {
    let original = basic_land_state(seed());
    let mut renamed = original.clone();
    let state = renamed.predecessor_v5.clone();
    let (old_id, location) = state
        .zones
        .locations
        .iter()
        .find(|(_, location)| location.zone == mtgml_model::ZoneKind::Battlefield)
        .map(|(id, location)| (*id, location.clone()))
        .expect("fixture has a battlefield object");
    let new_id = mtgml_model::GameObjectId(10_000);
    assert!(!state.zones.objects.contains_key(&new_id));

    // Keep every perspective's public identity stable while changing the
    // authoritative GameObjectId and all references to that incarnation.
    for identity in renamed
        .predecessor_v5
        .perspective_identities
        .players
        .values_mut()
    {
        if let Some(opaque) = identity.object_to_opaque.remove(&old_id) {
            identity.object_to_opaque.insert(new_id, opaque);
            assert_eq!(
                identity.opaque_to_object.insert(opaque, new_id),
                Some(old_id)
            );
        }
    }
    let object = renamed
        .predecessor_v5
        .zones
        .objects
        .remove(&old_id)
        .expect("fixture object exists");
    let mut object = object;
    object.id = new_id;
    renamed.predecessor_v5.zones.objects.insert(new_id, object);
    renamed
        .predecessor_v5
        .zones
        .locations
        .remove(&old_id)
        .expect("fixture location exists");
    renamed
        .predecessor_v5
        .zones
        .locations
        .insert(new_id, location);
    for object_id in renamed
        .predecessor_v5
        .zones
        .ordered_zones
        .values_mut()
        .flat_map(|objects| objects.iter_mut())
    {
        if *object_id == old_id {
            *object_id = new_id;
        }
    }
    renamed.predecessor_v5.allocators.next_object_id = mtgml_model::GameObjectId(new_id.0 + 1);
    if let Some(counts) = renamed.card_rules_state.counters.counters.remove(&old_id) {
        renamed
            .card_rules_state
            .counters
            .counters
            .insert(new_id, counts);
    }
    if let Some(face) = renamed.card_rules_state.faces.faces.remove(&old_id) {
        renamed.card_rules_state.faces.faces.insert(new_id, face);
    }

    original.validate().unwrap();
    renamed.validate().unwrap();

    let catalog = catalog_for(&[1, 2]);
    let (identity, semantic, rules) = execution_authority(&catalog);
    let project = |state: &EngineStatePartsV3| {
        crate::player_projection::project_magic_basic_land_observation(
            state,
            PlayerId(1),
            &identity,
            &semantic,
            &rules,
            &catalog,
        )
        .unwrap()
    };
    assert_eq!(
        mtgml_wire::encode_canonical(&project(&original)).unwrap(),
        mtgml_wire::encode_canonical(&project(&renamed)).unwrap()
    );
    let information = |state: &EngineStatePartsV3| {
        crate::player_projection::project_successor_information_state_v3(
            state,
            PlayerId(1),
            &identity,
            &semantic,
            &rules,
            &catalog,
        )
        .unwrap()
    };
    let (original_information, renamed_information) =
        (information(&original), information(&renamed));
    assert_eq!(original_information.digest, renamed_information.digest);
    assert_eq!(
        mtgml_wire::encode_canonical(&original_information).unwrap(),
        mtgml_wire::encode_canonical(&renamed_information).unwrap()
    );
}
