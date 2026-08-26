// Ownership fragment: read-only projection evidence. Included lexically by tests.rs so
// every identity remains tests::<name>.

#[test]
fn information_state_orders_active_and_retired_knowledge_jointly() {
    use mtgml_model::VisibleSequence;
    use mtgml_observation::PlayerKnownObjectV1;
    use mtgml_state::{
        KnowledgeAcquisitionCause, KnowledgeAcquisitionReason, KnowledgeHistoryChannel,
        KnowledgeInvalidationReason, KnowledgeInvalidationV2, KnownLocationFactV2,
        RetiredKnowledgeRecordV2,
    };
    let observed = |channel, sequence: u64, cause| KnowledgeAcquisitionReason::Observed {
        channel,
        sequence: VisibleSequence(sequence),
        cause,
    };

    let mut state =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: seed(),
        })
        .unwrap();

    // Interleave a retired record between two active records for player 1.
    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity
        .opaque_to_object
        .insert(mtgml_model::OpaqueObjectId(3), mtgml_model::GameObjectId(2));
    identity
        .object_to_opaque
        .insert(mtgml_model::GameObjectId(2), mtgml_model::OpaqueObjectId(3));
    identity.next_opaque_object_id = mtgml_model::OpaqueObjectId(4);
    identity
        .retired_object_ids
        .insert(mtgml_model::OpaqueObjectId(2));

    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    let hidden_location = mtgml_state::ZoneLocation {
        zone: mtgml_model::ZoneKind::Library,
        player: Some(PlayerId(2)),
        position: mtgml_state::ZonePosition::Top { offset: 0 },
        visibility: mtgml_state::VisibilityPartition::FaceDown,
        partition: None,
    };
    knowledge.retired.insert(
        mtgml_model::OpaqueObjectId(2),
        RetiredKnowledgeRecordV2 {
            opaque_object: mtgml_model::OpaqueObjectId(2),
            physical_card: None,
            card_definition: None,
            last_known_location: None,
            historical_locations: Vec::new(),
            acquisition: KnowledgeAcquisitionReason::InitialConfiguration,
            invalidation: KnowledgeInvalidationV2 {
                provenance: observed(
                    KnowledgeHistoryChannel::Public,
                    0,
                    KnowledgeAcquisitionCause::ExplicitReveal,
                ),
                reason: KnowledgeInvalidationReason::Shuffle,
            },
        },
    );
    knowledge.active.remove(&mtgml_model::OpaqueObjectId(2));
    knowledge.active.insert(
        mtgml_model::OpaqueObjectId(3),
        mtgml_state::KnowledgeRecordV2 {
            opaque_object: mtgml_model::OpaqueObjectId(3),
            physical_card: None,
            card_definition: Some(mtgml_model::CardDefinitionId(2)),
            known_location: Some(KnownLocationFactV2 {
                location: hidden_location,
                provenance: observed(
                    KnowledgeHistoryChannel::Public,
                    0,
                    KnowledgeAcquisitionCause::PublicEvent,
                ),
            }),
            acquisition: observed(
                KnowledgeHistoryChannel::Public,
                0,
                KnowledgeAcquisitionCause::PublicEvent,
            ),
            historical_locations: Vec::new(),
        },
    );

    let checkpoint = EnvironmentCheckpointV3::new(
        state.clone(),
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "synthetic-m2-memory".into(),
            semantic_version: "3".into(),
        },
    )
    .unwrap();
    let controller = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            checkpoint,
            config([PlayerId(1), PlayerId(2)]),
        )
        .unwrap(),
    );
    let endpoint = controller.bind_player(PlayerId(1)).unwrap();
    let information = endpoint.information_state().unwrap();
    information.validate().unwrap();
    let ids: Vec<u64> = information
        .retained_knowledge
        .iter()
        .map(|record| match record {
            PlayerKnownObjectV1::Active {
                opaque_object_id, ..
            }
            | PlayerKnownObjectV1::Retired {
                opaque_object_id, ..
            } => opaque_object_id.0,
        })
        .collect();
    assert_eq!(ids, vec![1, 2, 3]);
}

#[test]
fn provenance_is_preserved_through_projection_restore_and_fork() {
    let codec = CheckpointCodecIdentity {
        codec_id: "synthetic-m2-memory".into(),
        semantic_version: "3".into(),
    };
    let state = rich_provenance_state();
    let checkpoint = EnvironmentCheckpointV3::new(
        state.clone(),
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        codec.clone(),
    )
    .unwrap();

    let controller = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            checkpoint.clone(),
            config([PlayerId(1), PlayerId(2)]),
        )
        .unwrap(),
    );
    let endpoint = controller.bind_player(PlayerId(1)).unwrap();
    let projected = endpoint.information_state().unwrap();
    projected.validate().unwrap();
    let expected = projected_provenance(&projected);

    // The projection must not invent causes: every projected provenance
    // equals its authoritative counterpart.
    assert!(
        expected.contains(&(
            2u64,
            "invalidation/observed/Public/0/ExplicitReveal/Shuffle".to_string()
        )),
        "invalidation provenance was not preserved: {expected:?}"
    );
    assert!(
        expected.contains(&(
            2u64,
            "historical/observed/Private/0/OwnPrivateIdentity".to_string()
        )),
        "own_private_identity history was collapsed: {expected:?}"
    );
    assert!(
        expected.contains(&(3u64, "current/observed/Public/0/ExplicitReveal".to_string())),
        "explicit_reveal current fact was collapsed: {expected:?}"
    );
    assert!(
        !expected
            .iter()
            .any(|(_, text)| text.contains("PublicEvent")),
        "projection invented a public_event cause: {expected:?}"
    );

    // Checkpoint -> restore preserves exact provenance.
    let restored = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            checkpoint.clone(),
            config([PlayerId(1), PlayerId(2)]),
        )
        .unwrap(),
    );
    restored.restore(checkpoint.clone()).unwrap();
    let restored_endpoint = restored.bind_player(PlayerId(1)).unwrap();
    assert_eq!(
        projected_provenance(&restored_endpoint.information_state().unwrap()),
        expected
    );
    assert_eq!(restored.checkpoint().unwrap().state, state);

    // A fork preserves exact provenance.
    let fork = controller.fork().unwrap();
    let fork_endpoint = fork.bind_player(PlayerId(1)).unwrap();
    assert_eq!(
        projected_provenance(&fork_endpoint.information_state().unwrap()),
        expected
    );
    assert_eq!(fork.checkpoint().unwrap().state, state);
}

#[test]
fn projection_reads_are_pure_and_order_independent() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();

    let observation_bytes = mtgml_wire::encode_canonical(&p1.observation().unwrap()).unwrap();
    let information_bytes = mtgml_wire::encode_canonical(&p1.information_state().unwrap()).unwrap();
    let decision_bytes =
        mtgml_wire::encode_canonical(&p1.visible_decision().unwrap().unwrap()).unwrap();

    let before = public_fingerprint(&controller);

    // Out-of-order repeated reads.
    for _ in 0..3 {
        assert_eq!(
            mtgml_wire::encode_canonical(&p1.visible_decision().unwrap().unwrap()).unwrap(),
            decision_bytes
        );
        assert_eq!(
            mtgml_wire::encode_canonical(&p1.observation().unwrap()).unwrap(),
            observation_bytes
        );
        assert_eq!(
            mtgml_wire::encode_canonical(&p1.information_state().unwrap()).unwrap(),
            information_bytes
        );
        assert_eq!(public_fingerprint(&controller), before);
    }
}

#[test]
fn projection_perspective_and_revision_coherence_matrix() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let p2 = controller.bind_player(PlayerId(2)).unwrap();

    let observation = p1.observation().unwrap();
    let information = p1.information_state().unwrap();
    let decision = p1.visible_decision().unwrap().unwrap();

    assert_eq!(observation.perspective, PlayerId(1));
    assert_eq!(information.perspective, PlayerId(1));
    assert_eq!(decision.actor, PlayerId(1));
    assert_eq!(observation.state_revision, information.state_revision);
    assert_eq!(decision.state_revision, information.state_revision);
    // One canonical current observation.
    assert_eq!(
        mtgml_wire::encode_canonical(&observation).unwrap(),
        mtgml_wire::encode_canonical(&information.current_observation).unwrap()
    );

    // The other perspective sees the same revision but its own surface.
    let info2 = p2.information_state().unwrap();
    assert_eq!(info2.perspective, PlayerId(2));
    assert_eq!(info2.state_revision, information.state_revision);
}

#[test]
fn episode_status_does_not_change_the_information_digest() {
    use mtgml_model::EpisodeStatus;

    // Drive the synthetic chain to completion: no pending decision remains,
    // so both Running and Terminal statuses are valid environment contexts
    // over the identical authoritative state.
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let _ = submit_answer(&p1, order_entry_answer());
    let _ = submit_answer(&p1, number_answer(2));
    let _ = submit_answer(&p1, members_answer(&[0, 1]));
    let _ = submit_answer(&p1, order_answer(&[1, 0]));
    let final_state = controller.checkpoint().unwrap().state;

    let codec = CheckpointCodecIdentity {
        codec_id: "synthetic-m2-memory".into(),
        semantic_version: "3".into(),
    };
    let running = EnvironmentCheckpointV3::new(
        final_state.clone(),
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        codec.clone(),
    )
    .unwrap();
    let terminal = EnvironmentCheckpointV3::new(
        final_state.clone(),
        EpisodeStatus::Terminal {
            reason: TerminalReason::Concession,
            players: vec![mtgml_model::PlayerOutcome {
                player: PlayerId(1),
                result: mtgml_model::PlayerResult::Loss,
            }],
        },
        EnvironmentLimitCounters::default(),
        codec.clone(),
    )
    .unwrap();

    let running_env = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(running, config([PlayerId(1), PlayerId(2)]))
            .unwrap(),
    );
    let terminal_env = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            terminal,
            config([PlayerId(1), PlayerId(2)]),
        )
        .unwrap(),
    );
    let running_p1 = running_env.bind_player(PlayerId(1)).unwrap();
    let terminal_p1 = terminal_env.bind_player(PlayerId(1)).unwrap();

    // Identical information products including digest identity.
    assert_eq!(
        running_p1.information_state().unwrap(),
        terminal_p1.information_state().unwrap()
    );
    // Episode status itself differs on the step surface.
    assert_ne!(
        running_env.checkpoint().unwrap(),
        terminal_env.checkpoint().unwrap()
    );
}

#[test]
fn projection_bytes_survive_checkpoint_restore_and_equal_forks() {
    let source = environment_at_members_stage();
    let p1 = source.bind_player(PlayerId(1)).unwrap();

    let observation_bytes = mtgml_wire::encode_canonical(&p1.observation().unwrap()).unwrap();
    let information_bytes = mtgml_wire::encode_canonical(&p1.information_state().unwrap()).unwrap();
    let decision_bytes =
        mtgml_wire::encode_canonical(&p1.visible_decision().unwrap().unwrap()).unwrap();
    let checkpoint_before = source.checkpoint().unwrap();

    let restored = TrustedEnvironmentController::new(backend());
    restored.restore(checkpoint_before.clone()).unwrap();
    let restored_p1 = restored.bind_player(PlayerId(1)).unwrap();
    assert_eq!(
        mtgml_wire::encode_canonical(&restored_p1.observation().unwrap()).unwrap(),
        observation_bytes
    );
    assert_eq!(
        mtgml_wire::encode_canonical(&restored_p1.information_state().unwrap()).unwrap(),
        information_bytes
    );
    assert_eq!(
        mtgml_wire::encode_canonical(&restored_p1.visible_decision().unwrap().unwrap()).unwrap(),
        decision_bytes
    );

    let fork = source.fork().unwrap();
    let fork_p1 = fork.bind_player(PlayerId(1)).unwrap();
    assert_eq!(
        mtgml_wire::encode_canonical(&fork_p1.observation().unwrap()).unwrap(),
        observation_bytes
    );
    assert_eq!(
        mtgml_wire::encode_canonical(&fork_p1.information_state().unwrap()).unwrap(),
        information_bytes
    );
    assert_eq!(
        mtgml_wire::encode_canonical(&fork_p1.visible_decision().unwrap().unwrap()).unwrap(),
        decision_bytes
    );

    // Projection calls do not mutate the checkpoint fingerprint.
    assert_eq!(source.checkpoint().unwrap(), checkpoint_before);
}

#[test]
fn visible_decision_exposes_no_trusted_identities_or_internals() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let request = p1.visible_decision().unwrap().unwrap();
    let bytes = mtgml_wire::encode_canonical(&request).unwrap();
    let rendered = String::from_utf8(bytes.clone()).unwrap();

    // Structural forbidden-key checks over the serialized graph.
    // `player_decision_id` is a legitimate public key; the forbidden keys
    // are matched as exact quoted JSON keys.
    for forbidden_key in [
        "\"decision_id\"",
        "\"continuation_id\"",
        "\"game_object_id\"",
        "\"physical_card_id\"",
        "\"ability_instance_id\"",
        "\"rule_event_id\"",
        "\"trusted_binding\"",
        "\"root_seed\"",
        "\"checkpoint_digest\"",
        "\"full_state_digest\"",
        "\"stream_key\"",
        "\"next_raw_u64\"",
    ] {
        assert!(
            !rendered.contains(forbidden_key),
            "visible decision leaked forbidden key {forbidden_key}"
        );
    }

    // Paired states: unrelated trusted/global values must not move the
    // public bytes.
    let mut variant =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: seed(),
        })
        .unwrap();
    variant.allocators.next_effect_id = mtgml_model::EffectInstanceId(500);
    variant.allocators.next_trigger_id = mtgml_model::TriggerInstanceId(900);
    let checkpoint = EnvironmentCheckpointV3::new(
        variant,
        EpisodeStatus::Running,
        EnvironmentLimitCounters::default(),
        CheckpointCodecIdentity {
            codec_id: "synthetic-m2-memory".into(),
            semantic_version: "3".into(),
        },
    )
    .unwrap();
    let other_controller = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(
            checkpoint,
            config([PlayerId(1), PlayerId(2)]),
        )
        .unwrap(),
    );
    let other_p1 = other_controller.bind_player(PlayerId(1)).unwrap();
    let other_request = other_p1.visible_decision().unwrap().unwrap();
    assert_eq!(
        mtgml_wire::encode_canonical(&request).unwrap(),
        mtgml_wire::encode_canonical(&other_request).unwrap(),
        "unrelated trusted/global history changed the visible decision bytes"
    );
}

#[test]
fn observation_equals_information_state_current_observation_bytes() {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let observation = p1.observation().unwrap();
    let information = p1.information_state().unwrap();
    assert_eq!(
        mtgml_wire::encode_canonical(&observation).unwrap(),
        mtgml_wire::encode_canonical(&information.current_observation).unwrap()
    );
}

#[test]
fn occurrence_projection_resolves_old_via_before_and_new_via_after() {
    let (before, result) = tracked_incarnation_product().unwrap();
    let envelopes = crate::lifecycle_projection::project_occurrence_envelopes(
        &before,
        &result.next_state,
        &result.events,
    )
    .unwrap();
    let p1 = &envelopes[&PlayerId(1)];
    assert_eq!(p1.len(), 2);
    // Appearance: old unknown, new resolves through the AFTER mapping.
    match &p1[0].event {
        mtgml_observation::ObservedEventKindV2::ObjectMoved {
            old_object: None,
            new_object: Some(opaque),
            ..
        } => {
            assert_eq!(*opaque, OpaqueObjectId(2));
        }
        other => panic!("unexpected first envelope {other:?}"),
    }
    // Tracked disappearance: old resolves through the BEFORE mapping even
    // though the AFTER map no longer contains that incarnation.
    match &p1[1].event {
        mtgml_observation::ObservedEventKindV2::ObjectMoved {
            old_object: Some(opaque),
            new_object: None,
            from,
            to,
        } => {
            assert_eq!(*opaque, OpaqueObjectId(2));
            assert_eq!(*from, mtgml_model::ZoneKind::Battlefield);
            assert_eq!(*to, mtgml_model::ZoneKind::Hand);
        }
        other => panic!("unexpected second envelope {other:?}"),
    }
    assert_eq!(p1[0].sequence.0, 1);
    assert_eq!(p1[1].sequence.0, 2);
    assert_eq!(p1[0].state_revision, result.next_state.revision);
}

#[test]
fn repeated_projection_is_pure_and_stable() {
    let (before, result) = tracked_incarnation_product().unwrap();
    let before_digest = before.digest().unwrap();
    let after_digest = result.next_state.digest().unwrap();
    let first = crate::lifecycle_projection::project_occurrence_envelopes(
        &before,
        &result.next_state,
        &result.events,
    )
    .unwrap();
    let second = crate::lifecycle_projection::project_occurrence_envelopes(
        &before,
        &result.next_state,
        &result.events,
    )
    .unwrap();
    assert_eq!(first, second);
    assert_eq!(before.digest().unwrap(), before_digest);
    assert_eq!(result.next_state.digest().unwrap(), after_digest);
}
