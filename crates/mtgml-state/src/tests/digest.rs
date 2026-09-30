// Ownership fragment: canonical digest known-answer/mutation evidence. Included lexically by tests.rs so
// every identity remains tests::<name>.










#[test]
fn card_rules_state_rejects_empty_counter_maps() {
    let mut state = crate::CardRulesAuthoritativeStateV1::default();
    state.counters.counters.insert(
        mtgml_model::GameObjectId(1),
        std::collections::BTreeMap::new(),
    );
    assert!(state.validate().is_err());
    assert!(state.canonical_value().is_err());
    let mut current = current_state(&synthetic_state());
    current.card_rules_state = state;
    assert!(calculate_full_state_digest_v7(&current).is_err());
}












#[test]
fn v7_digest_changes_for_each_state_component_mutation() {
    type Mutation = (&'static str, fn(&mut EngineState));
    let mutations: Vec<Mutation> = vec![
        ("revision_and_pending_revision", |state| {
            state.revision = StateRevision(1);
            if let Some(pending) = state.execution.pending_decision.as_mut() {
                pending.request.state_revision = StateRevision(1);
            }
        }),
        ("core_life", |state| {
            state.core.players.get_mut(&PlayerId(1)).unwrap().life = 39;
        }),
        ("core_has_lost", |state| {
            state.core.players.get_mut(&PlayerId(2)).unwrap().has_lost = true;
        }),
        ("core_active_player", |state| {
            state.core.active_player = PlayerId(2);
        }),
        ("core_priority", |state| {
            state.core.priority = PriorityState::HeldBy {
                player: PlayerId(2),
                consecutive_passes: 0,
            };
        }),
        ("core_turn_number", |state| {
            state.core.turn_number += 1;
        }),
        ("core_position", |state| {
            state.core.position = TurnPosition::Beginning {
                step: BeginningStep::Upkeep,
            };
        }),
        ("core_priority_pass_count", |state| {
            state.core.priority = PriorityState::HeldBy {
                player: PlayerId(1),
                consecutive_passes: 1,
            };
        }),
        ("combat_presence", |state| {
            state.combat = Some(CombatState {
                defending_player: PlayerId(2),
                attackers: vec![GameObjectId(1)],
                damage_step_completed: false,
                blocked_attackers: BTreeSet::new(),
                blockers: BTreeMap::from([(GameObjectId(1), None)]),
            });
        }),
        ("zone_object_tapped", |state| {
            state
                .zones
                .objects
                .get_mut(&GameObjectId(1))
                .unwrap()
                .tapped = true;
        }),
        ("zone_object_face_down", |state| {
            state
                .zones
                .objects
                .get_mut(&GameObjectId(1))
                .unwrap()
                .face_down = true;
        }),
        ("zone_object_controller", |state| {
            state
                .zones
                .objects
                .get_mut(&GameObjectId(1))
                .unwrap()
                .controller = PlayerId(2);
        }),
        ("zone_object_owner", |state| {
            state.zones.objects.get_mut(&GameObjectId(1)).unwrap().owner = PlayerId(2);
        }),
        ("zone_object_physical_card", |state| {
            state
                .zones
                .objects
                .get_mut(&GameObjectId(1))
                .unwrap()
                .physical_card = None;
            for knowledge in state.knowledge.players.values_mut() {
                if let Some(record) = knowledge.active.get_mut(&OpaqueObjectId(1)) {
                    record.physical_card = None;
                }
            }
        }),
        ("zone_object_card_definition", |state| {
            state
                .zones
                .objects
                .get_mut(&GameObjectId(1))
                .unwrap()
                .card_definition = CardDefinitionId(9);
            for knowledge in state.knowledge.players.values_mut() {
                if let Some(record) = knowledge.active.get_mut(&OpaqueObjectId(1)) {
                    record.card_definition = Some(CardDefinitionId(9));
                }
            }
        }),
        ("zone_location_zone", |state| {
            let graveyard = ZoneLocation {
                zone: ZoneKind::Graveyard,
                ..public_location()
            };
            state
                .zones
                .locations
                .insert(GameObjectId(1), graveyard.clone());
            for knowledge in state.knowledge.players.values_mut() {
                if let Some(record) = knowledge.active.get_mut(&OpaqueObjectId(1)) {
                    if let Some(current) = record.known_location.as_mut() {
                        current.location = graveyard.clone();
                    }
                }
            }
        }),
        ("allocator_next_object_id", |state| {
            state.allocators.next_object_id = GameObjectId(4);
        }),
        ("allocator_next_ability_id", |state| {
            state.allocators.next_ability_id = AbilityInstanceId(2);
        }),
        ("allocator_next_stack_object_id", |state| {
            state.allocators.next_stack_object_id = StackObjectId(2);
        }),
        ("allocator_next_effect_id", |state| {
            state.allocators.next_effect_id = mtgml_model::EffectInstanceId(2);
        }),
        ("allocator_next_trigger_id", |state| {
            state.allocators.next_trigger_id = TriggerInstanceId(2);
        }),
        ("allocator_next_decision_id", |state| {
            state.allocators.next_decision_id = DecisionId(3);
        }),
        ("allocator_next_continuation_id", |state| {
            state.allocators.next_continuation_id = ContinuationId(2);
        }),
        ("allocator_next_rule_event_id", |state| {
            state.allocators.next_rule_event_id = mtgml_model::RuleEventId(2);
        }),
        ("random_root_seed", |state| {
            let seed = state.random.root_seed.as_bytes();
            let mut hex = String::with_capacity(64);
            for byte in seed {
                std::fmt::Write::write_fmt(&mut hex, format_args!("{byte:02x}")).unwrap();
            }
            let last = hex.pop().unwrap();
            hex.push(if last == '1' { '2' } else { '1' });
            state.random.root_seed = RootSeed256::from_lower_hex(&hex).unwrap();
        }),
        ("random_stream_cursor", |state| {
            let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
            let next = state.random.lookup_stream(&key).unwrap().next_raw_u64 + 1;
            state
                .random
                .set_cursor(&key, RandomStreamCursorV1 { next_raw_u64: next })
                .unwrap();
        }),
        ("random_additional_stream", |state| {
            state
                .random
                .streams
                .entry(RandomStreamKeyV1::player_scoped(
                    RandomStreamKindV1::SyntheticM1,
                    1,
                ))
                .or_default();
        }),
        ("knowledge_acquisition_provenance", |state| {
            let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
            let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
            let provenance = observed(
                KnowledgeHistoryChannel::Public,
                0,
                KnowledgeAcquisitionCause::PublicEvent,
            );
            record.acquisition = provenance;
            record.known_location.as_mut().unwrap().provenance = provenance;
        }),
        ("knowledge_provenance_cause_only", |state| {
            let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
            let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
            let provenance = observed(
                KnowledgeHistoryChannel::Public,
                0,
                KnowledgeAcquisitionCause::ExplicitReveal,
            );
            record.acquisition = provenance;
            record.known_location.as_mut().unwrap().provenance = provenance;
        }),
        ("knowledge_known_location", |state| {
            let graveyard = ZoneLocation {
                zone: ZoneKind::Graveyard,
                ..public_location()
            };
            state
                .zones
                .locations
                .insert(GameObjectId(1), graveyard.clone());
            for knowledge in state.knowledge.players.values_mut() {
                if let Some(record) = knowledge.active.get_mut(&OpaqueObjectId(1)) {
                    if let Some(current) = record.known_location.as_mut() {
                        current.location = graveyard.clone();
                    }
                }
            }
        }),
        ("knowledge_private_acquisition", |state| {
            let knowledge = state.knowledge.players.get_mut(&PlayerId(2)).unwrap();
            let record = knowledge.active.get_mut(&OpaqueObjectId(2)).unwrap();
            let provenance = observed(
                KnowledgeHistoryChannel::Private,
                0,
                KnowledgeAcquisitionCause::PrivateLook,
            );
            record.acquisition = provenance;
            record.known_location.as_mut().unwrap().provenance = provenance;
        }),
        ("knowledge_historical_location", |state| {
            let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
            let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
            record.known_location = None;
            record.historical_locations.push(fact(
                    public_location(),
                    observed(
                        KnowledgeHistoryChannel::Public,
                        0,
                        KnowledgeAcquisitionCause::PublicEvent,
                    ),
                ));
        }),
        ("knowledge_retired_record", |state| {
            let identity = state
                .perspective_identities
                .players
                .get_mut(&PlayerId(1))
                .unwrap();
            identity.next_opaque_object_id = OpaqueObjectId(6);
            identity.retired_object_ids.insert(OpaqueObjectId(5));
            let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
            knowledge
                .retired
                .insert(OpaqueObjectId(5), retired_record(OpaqueObjectId(5)));
        }),
        ("knowledge_next_visible_sequence", |state| {
            let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
            knowledge.next_visible_sequence = VisibleSequence(2);
        }),
        ("identity_object_mapping", |state| {
            let identity = state
                .perspective_identities
                .players
                .get_mut(&PlayerId(1))
                .unwrap();
            identity
                .opaque_to_object
                .insert(OpaqueObjectId(2), GameObjectId(2));
            identity
                .object_to_opaque
                .insert(GameObjectId(2), OpaqueObjectId(2));
            identity.next_opaque_object_id = OpaqueObjectId(3);
        }),
        ("identity_next_player_decision_id", |state| {
            let identity = state
                .perspective_identities
                .players
                .get_mut(&PlayerId(1))
                .unwrap();
            identity.next_player_decision_id = mtgml_model::PlayerDecisionIdV1(3);
        }),
        ("format_commander", |state| {
            state.format = FormatState::Commander {
                state: CommanderState {
                    designations: BTreeMap::from([(PlayerId(1), vec![PhysicalCardId(1)])]),
                    cast_counts: BTreeMap::new(),
                    damage: BTreeMap::new(),
                },
            };
        }),
    ];

    let baseline = synthetic_state();
    let baseline_digest = v7_digest(&baseline);
    assert!(!mutations.is_empty());
    let mut seen = BTreeMap::new();
    for (name, mutate) in mutations {
        let mut changed = synthetic_state();
        mutate(&mut changed);
        validate_engine_state(&changed)
            .unwrap_or_else(|error| panic!("mutation {name} must stay valid: {error}"));
        let changed_digest = v7_digest(&changed);
        assert_ne!(
            baseline_digest, changed_digest,
            "mutation {name} must change the V7 digest"
        );
        // Two mutations may reach the same state; distinct states must not
        // share a digest.
        if let Some((other, other_state)) = seen.insert(changed_digest, (name, changed.clone())) {
            assert_eq!(
                other_state, changed,
                "mutations {other} and {name} give the same V7 digest"
            );
        }
    }
}




#[test]
fn v7_digest_binds_combat_inner_values() {
    let mut baseline = synthetic_state();
    baseline.combat = Some(CombatState {
        defending_player: PlayerId(2),
        attackers: vec![GameObjectId(1), GameObjectId(2)],
        damage_step_completed: false,
        blocked_attackers: BTreeSet::new(),
        blockers: BTreeMap::from([
            (GameObjectId(1), None),
            (GameObjectId(2), None),
        ]),
    });
    baseline.core.position = TurnPosition::Combat {
        step: crate::CombatStep::CombatDamage,
    };
    validate_engine_state(&baseline).unwrap();
    let baseline_digest = v7_digest(&baseline);

    let mut changed = baseline.clone();
    changed.combat.as_mut().unwrap().defending_player = PlayerId(1);
    validate_engine_state(&changed).unwrap();
    assert_ne!(baseline_digest, v7_digest(&changed));

    let mut changed = baseline.clone();
    changed.combat.as_mut().unwrap().blocked_attackers.insert(GameObjectId(1));
    validate_engine_state(&changed).unwrap();
    assert_ne!(baseline_digest, v7_digest(&changed));

    let mut changed = baseline.clone();
    changed.combat.as_mut().unwrap().damage_step_completed = true;
    validate_engine_state(&changed).unwrap();
    assert_ne!(baseline_digest, v7_digest(&changed));
}

#[test]
fn v7_digest_binds_each_card_rules_family() {
    type Mutation = (&'static str, fn(&mut EngineStatePartsV3));
    let mutations: Vec<Mutation> = vec![
        ("mana_unrestricted", |state| {
            state.card_rules_state.mana.pools.get_mut(&PlayerId(1)).unwrap().unrestricted[0] = 1;
        }),
        ("mana_creature_spell_only", |state| {
            state.card_rules_state.mana.pools.get_mut(&PlayerId(1)).unwrap().creature_spell_only
                [5] = 1;
        }),
        ("history_land_plays_used", |state| {
            state
                .card_rules_state
                .turn_history
                .players
                .get_mut(&PlayerId(1))
                .unwrap()
                .land_plays_used = 1;
        }),
        ("history_lost_life", |state| {
            state
                .card_rules_state
                .turn_history
                .players
                .get_mut(&PlayerId(2))
                .unwrap()
                .lost_life_this_turn = true;
        }),
        ("counter", |state| {
            state
                .card_rules_state
                .counters
                .counters
                .insert(GameObjectId(1), BTreeMap::from([(CounterKindV1::Lore, 1)]));
        }),
        ("face", |state| {
            // Face authority must cover every live object.
            state.card_rules_state.faces.faces = BTreeMap::from([
                (GameObjectId(1), 0),
                (GameObjectId(2), 0),
            ]);
        }),
        ("ability", |state| {
            state.card_rules_state.abilities.by_instance.insert(
                AbilityInstanceId(1),
                AbilityAuthorityV1 {
                    source: GameObjectId(1),
                    ability_key: 0,
                },
            );
            state.predecessor_v5.allocators.next_ability_id = AbilityInstanceId(2);
            state.card_rules_state.faces.faces = BTreeMap::from([
                (GameObjectId(1), 0),
                (GameObjectId(2), 0),
            ]);
        }),
        ("ability_identity_mapping", |state| {
            state.card_rules_state.abilities.by_instance.insert(
                AbilityInstanceId(1),
                AbilityAuthorityV1 {
                    source: GameObjectId(1),
                    ability_key: 0,
                },
            );
            state.predecessor_v5.allocators.next_ability_id = AbilityInstanceId(2);
            state.card_rules_state.faces.faces = BTreeMap::from([
                (GameObjectId(1), 0),
                (GameObjectId(2), 0),
            ]);
            let identity = state
                .predecessor_v5
                .perspective_identities
                .players
                .get_mut(&PlayerId(1))
                .unwrap();
            identity
                .opaque_to_ability
                .insert(OpaqueAbilityId(1), AbilityInstanceId(1));
            identity
                .ability_to_opaque
                .insert(AbilityInstanceId(1), OpaqueAbilityId(1));
            identity.next_opaque_ability_id = OpaqueAbilityId(2);
        }),
    ];
    let baseline = current_state(&synthetic_state());
    let baseline_digest = calculate_full_state_digest_v7(&baseline).unwrap();
    let mut seen = BTreeMap::new();
    for (name, mutate) in mutations {
        let mut changed = baseline.clone();
        mutate(&mut changed);
        changed
            .validate()
            .unwrap_or_else(|error| panic!("mutation {name} must stay valid: {error}"));
        let changed_digest = calculate_full_state_digest_v7(&changed).unwrap();
        assert_ne!(
            baseline_digest, changed_digest,
            "mutation {name} must change the V7 digest"
        );
        if let Some(other) = seen.insert(changed_digest, name) {
            panic!("mutations {other} and {name} give the same V7 digest");
        }
    }
}

#[test]
fn knowledge_history_is_digested_without_a_player_level_aggregate() {
    let mut state = synthetic_state();
    let knowledge = state.knowledge.players.get_mut(&PlayerId(2)).unwrap();
    let record = knowledge.active.get_mut(&OpaqueObjectId(2)).unwrap();
    let location = record.known_location.clone().unwrap().location;
    record.known_location = None;
    record.historical_locations.push(fact(
        location,
        observed(
            KnowledgeHistoryChannel::Private,
            0,
            KnowledgeAcquisitionCause::PrivateLook,
        ),
    ));
    validate_engine_state(&state).unwrap();
    let with_history = v7_digest(&state);
    let mut stripped = state.clone();
    stripped
        .knowledge
        .players
        .get_mut(&PlayerId(2))
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(2))
        .unwrap()
        .historical_locations
        .clear();
    validate_engine_state(&stripped).unwrap();
    assert_ne!(with_history, v7_digest(&stripped));
}



#[test]
fn historical_private_look_provenance_is_bound_into_the_digest() {
    let mut state = synthetic_state();
    let knowledge = state.knowledge.players.get_mut(&PlayerId(2)).unwrap();
    let record = knowledge.active.get_mut(&OpaqueObjectId(2)).unwrap();
    let location = record.known_location.clone().unwrap().location;
    record.known_location = None;
    record.historical_locations.push(fact(
        location,
        observed(
            KnowledgeHistoryChannel::Private,
            0,
            KnowledgeAcquisitionCause::PrivateLook,
        ),
    ));
    validate_engine_state(&state).unwrap();
    assert!(digest_payload_texts(&state).contains(&"private_look".to_string()));

    // Changing only the retained cause changes the V7 digest.
    let baseline_digest = v7_digest(&state);
    let mut changed = state.clone();
    let knowledge = changed.knowledge.players.get_mut(&PlayerId(2)).unwrap();
    let record = knowledge.active.get_mut(&OpaqueObjectId(2)).unwrap();
    record.historical_locations[0].provenance = observed(
        KnowledgeHistoryChannel::Private,
        0,
        KnowledgeAcquisitionCause::OwnPrivateIdentity,
    );
    validate_engine_state(&changed).unwrap();
    assert_ne!(baseline_digest, v7_digest(&changed));
}

#[test]
fn explicit_reveal_is_not_collapsed_to_public_event() {
    let mut state = synthetic_state();
    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
    let location = record.known_location.clone().unwrap().location;
    record.known_location = None;
    record.historical_locations.push(fact(
        location,
        observed(
            KnowledgeHistoryChannel::Public,
            0,
            KnowledgeAcquisitionCause::ExplicitReveal,
        ),
    ));
    validate_engine_state(&state).unwrap();
    let texts = digest_payload_texts(&state);
    assert!(texts.contains(&"explicit_reveal".to_string()));
}

#[test]
fn own_private_identity_is_not_collapsed_to_private_look() {
    let mut state = synthetic_state();
    let knowledge = state.knowledge.players.get_mut(&PlayerId(2)).unwrap();
    let record = knowledge.active.get_mut(&OpaqueObjectId(2)).unwrap();
    let location = record.known_location.clone().unwrap().location;
    record.known_location = None;
    record.historical_locations.push(fact(
        location,
        observed(
            KnowledgeHistoryChannel::Private,
            0,
            KnowledgeAcquisitionCause::OwnPrivateIdentity,
        ),
    ));
    validate_engine_state(&state).unwrap();
    assert!(digest_payload_texts(&state).contains(&"own_private_identity".to_string()));
}

#[test]
fn invalidation_provenance_is_preserved_exactly() {
    let mut state = synthetic_state();
    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity.next_opaque_object_id = OpaqueObjectId(6);
    identity.retired_object_ids.insert(OpaqueObjectId(5));
    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    knowledge
        .retired
        .insert(OpaqueObjectId(5), retired_record(OpaqueObjectId(5)));
    validate_engine_state(&state).unwrap();

    let texts = digest_payload_texts(&state);
    assert!(texts.contains(&"explicit_reveal".to_string()));
    assert!(texts.contains(&"shuffle".to_string()));

    // Mutating only the invalidation provenance changes the digest.
    let baseline_digest = v7_digest(&state);
    let mut changed = state.clone();
    let knowledge = changed.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    let record = knowledge.retired.get_mut(&OpaqueObjectId(5)).unwrap();
    record.invalidation.provenance = observed(
        KnowledgeHistoryChannel::Public,
        0,
        KnowledgeAcquisitionCause::PublicEvent,
    );
    validate_engine_state(&changed).unwrap();
    assert_ne!(baseline_digest, v7_digest(&changed));
}

#[test]
fn historical_monotonicity_ignores_unsequenced_provenance() {
    let build = |provenances: Vec<KnowledgeAcquisitionReason>| {
        let mut state = synthetic_state();
        let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
        let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
        let location = record.known_location.clone().unwrap().location;
        record.known_location = None;
        record.historical_locations = provenances
            .into_iter()
            .map(|provenance| fact(location.clone(), provenance))
            .collect();
        state
    };

    // An unsequenced initial fact followed by an observed fact is valid.
    let valid = build(vec![
        KnowledgeAcquisitionReason::InitialConfiguration,
        observed(
            KnowledgeHistoryChannel::Public,
            0,
            KnowledgeAcquisitionCause::PublicEvent,
        ),
    ]);
    validate_engine_state(&valid).unwrap();

    // An initial location after observed history is invalid.
    let invalid = build(vec![
        observed(
            KnowledgeHistoryChannel::Public,
            0,
            KnowledgeAcquisitionCause::PublicEvent,
        ),
        KnowledgeAcquisitionReason::InitialConfiguration,
    ]);
    assert_eq!(
        validate_engine_state(&invalid),
        Err(EngineStateViolation::EngineStateShape(
            EngineStateShapeViolation::Knowledge
        ))
    );
}
