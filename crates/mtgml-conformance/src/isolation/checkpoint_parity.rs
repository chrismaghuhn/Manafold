//! Checkpoint→restore information-parity evidence for M2.G G.5.
//!
//! Restoring a captured checkpoint onto the live controller must return the
//! semantic, environment, and player groups to the checkpoint-time bytes
//! while the replay recorder restarts as a fresh empty segment anchored at
//! exactly the restored checkpoint identity, and an identical resumed input
//! submitted through the real player endpoint of the original environment
//! and of a twin rebuilt from the same checkpoint onto a fresh backend must
//! produce byte-identical transition products and successor fingerprints.
//! Both scenario classes are exercised: decision-rich (live mid-stage
//! continuation chain) and information-rich (retained knowledge with private
//! provenance and history, remapped and retired opaque identities, advanced
//! perspective cursors). A corrupted checkpoint must fail closed without
//! touching the live environment. Every M2.G gate remains `NOT_RUN`;
//! nothing here is a gate verdict.

#[cfg(test)]
pub(crate) mod support {
    use crate::isolation::paired::test_support::accepted_entry_submission;
    use crate::isolation::paired::{
        base_pair_state, spawn_environment, synthetic_environment_config,
    };
    use crate::isolation::HarnessError;
    use mtgml_decision::{
        DecisionAnswerV2, DecisionDomainV2, DecisionResponseV2, PlayerDecisionRequestV2,
        DECISION_RESPONSE_V2_SCHEMA,
    };
    use mtgml_environment::{
        EnvironmentCheckpointV3, PlayerEndpoint, PlayerEndpointHandle,
        SyntheticM1EnvironmentConfig, TrustedEnvironmentController,
    };
    use mtgml_model::{
        CardDefinitionId, EnvironmentLimitCounters, EpisodeStatus, GameObjectId, OpaqueObjectId,
        PhysicalCardId, PlayerId, StateRevision, VisibleSequence, ZoneKind,
    };
    use mtgml_observation::PlayerStepSubmissionV1;
    use mtgml_random::{RandomStreamKeyV1, RandomStreamKindV1};
    use mtgml_replay::InitialEnvironmentIdentityV3;
    use mtgml_rules::fixture_support::{FixtureTransition, PlannedOccurrence};
    use mtgml_rules::PerspectiveObservationPolicyV1;
    use mtgml_state::{
        validate_engine_state, AssemblyStageV2, ContinuationPayloadV2, EngineState, GameObject,
        IdentityMutationV1, KnowledgeAcquisitionCause, KnowledgeAcquisitionReason,
        KnowledgeHistoryChannel, KnowledgeInvalidationReason, KnowledgeMutationV1,
        KnownLocationFactV2, PerspectiveLifecycleAuditV1, PerspectiveLifecycleMutationV1,
        VisibilityPartition, ZoneLocation, ZonePosition, SYNTHETIC_COUNT_MAX, SYNTHETIC_COUNT_MIN,
    };

    pub(crate) const P1: PlayerId = PlayerId(1);
    pub(crate) const P2: PlayerId = PlayerId(2);

    const SEED_HEX_DECISION_RICH: &str =
        "7777777777777777777777777777777777777777777777777777777777777777";
    const SEED_HEX_INFORMATION_RICH: &str =
        "8888888888888888888888888888888888888888888888888888888888888888";

    /// The identical resumed answer of the equal-input phases; a second,
    /// different-but-valid count exists for divergence evidence.
    pub(crate) const EQUAL_COUNT_VALUE: i64 = 2;
    pub(crate) const DIVERGENT_COUNT_VALUE: i64 = 3;

    /// The one perspective whose lifecycle occurrences enrich the
    /// information-rich scenario beyond its construction-time defaults.
    pub(crate) fn config() -> SyntheticM1EnvironmentConfig {
        synthetic_environment_config([P1, P2])
    }

    /// The two declared G.5 scenario classes sharing one harness pipeline.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum ParityScenario {
        DecisionRich,
        InformationRich,
    }

    fn battlefield() -> ZoneLocation {
        ZoneLocation {
            zone: ZoneKind::Battlefield,
            player: None,
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        }
    }

    fn hidden_hand(player: PlayerId) -> ZoneLocation {
        ZoneLocation {
            zone: ZoneKind::Hand,
            player: Some(player),
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::OwnerOnly,
            partition: None,
        }
    }

    fn exile_location() -> ZoneLocation {
        ZoneLocation {
            zone: ZoneKind::Exile,
            player: None,
            position: ZonePosition::Unordered,
            visibility: VisibilityPartition::Public,
            partition: None,
        }
    }

    fn observed(
        sequence: u64,
        channel: KnowledgeHistoryChannel,
        cause: KnowledgeAcquisitionCause,
    ) -> KnowledgeAcquisitionReason {
        KnowledgeAcquisitionReason::Observed {
            channel,
            sequence: VisibleSequence(sequence),
            cause,
        }
    }

    fn occurrence(
        perspective: PlayerId,
        sequence: u64,
        identity: IdentityMutationV1,
        knowledge: Option<KnowledgeMutationV1>,
        observation: PerspectiveObservationPolicyV1,
    ) -> PlannedOccurrence {
        PlannedOccurrence {
            lifecycle: PerspectiveLifecycleAuditV1 {
                perspective,
                sequence: VisibleSequence(sequence),
                mutation: PerspectiveLifecycleMutationV1 {
                    identity,
                    knowledge,
                },
            },
            observation,
        }
    }

    fn fixture_transition(error: mtgml_rules::KernelExecutionError) -> HarnessError {
        panic!("fixture transition rejected: {error:?}");
        #[allow(unreachable_code)]
        HarnessError::FixtureTransitionRejected
    }

    fn global_stream_cursor(state: &EngineState) -> Result<u64, HarnessError> {
        let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
        state
            .random
            .streams
            .get(&key)
            .map(|cursor| cursor.next_raw_u64)
            .ok_or(HarnessError::TransformPreconditionViolated)
    }

    /// DECISION-RICH S*: one accepted entry submission through the REAL
    /// player endpoint creates the live continuation chain advanced to the
    /// ChooseCount mid-stage under a non-default allocated
    /// `PlayerDecisionIdV1`, with progressed revision, consumed RNG cursor,
    /// and non-default limit counters.
    pub(crate) fn decision_rich_spawned(
    ) -> Result<(TrustedEnvironmentController, [PlayerEndpointHandle; 2]), HarnessError> {
        let (controller, endpoints) =
            spawn_environment(base_pair_state(SEED_HEX_DECISION_RICH)?, &config())?;
        let entry_step = accepted_entry_submission(&endpoints[0])?;
        assert_eq!(
            entry_step.submission,
            PlayerStepSubmissionV1::Accepted,
            "the entry transition must be accepted"
        );

        let request = visible_request(&endpoints[0])?;
        match request.decision {
            DecisionDomainV2::ChooseNumber { minimum, maximum } => assert_eq!(
                (minimum, maximum),
                (
                    i64::from(SYNTHETIC_COUNT_MIN),
                    i64::from(SYNTHETIC_COUNT_MAX)
                ),
                "the live continuation must expose the ChooseCount mid-stage"
            ),
            _ => return Err(HarnessError::TransformPreconditionViolated),
        }
        assert!(
            request.player_decision_id.0 >= 2,
            "a non-default player decision id must be allocated"
        );

        let checkpoint = controller
            .checkpoint()
            .map_err(|_| HarnessError::ControllerService)?;
        let pending = checkpoint
            .state
            .execution
            .pending_decision
            .as_ref()
            .ok_or(HarnessError::TransformPreconditionViolated)?;
        let continuation_id = pending
            .request
            .continuation_id
            .ok_or(HarnessError::TransformPreconditionViolated)?;
        let record = checkpoint
            .state
            .execution
            .continuations
            .get(&continuation_id)
            .ok_or(HarnessError::TransformPreconditionViolated)?;
        assert!(matches!(
            record.payload,
            ContinuationPayloadV2::SyntheticM2Assembly {
                stage: AssemblyStageV2::ChooseCount,
                selected_count: None,
                ..
            }
        ));
        assert!(checkpoint.state.revision > StateRevision(0));
        assert!(checkpoint.limit_counters.accepted_transitions >= 1);
        assert!(checkpoint.limit_counters.decisions_submitted >= 1);
        let base_cursor = global_stream_cursor(&base_pair_state(SEED_HEX_DECISION_RICH)?)?;
        assert!(
            global_stream_cursor(&checkpoint.state)? > base_cursor,
            "the accepted entry must consume authoritative randomness"
        );
        Ok((controller, endpoints))
    }

    /// INFORMATION-RICH S*: composed through this crate's own fixture
    /// program over six planned occurrences so that P1 acquires knowledge
    /// with public-reveal provenance plus an ordered history, remaps the
    /// opaque identity onto a hidden incarnation, privately looks at it
    /// (private-channel provenance), and retires it by explicit forget,
    /// while P2 independently acquires the public reveal and then privately
    /// accounts its own hand content. Perspective allocators and
    /// visible-sequence cursors advance beyond their construction-time
    /// defaults on both sides.
    pub(crate) fn information_rich_state() -> Result<EngineState, HarnessError> {
        let mut before = base_pair_state(SEED_HEX_INFORMATION_RICH)?;
        // Unseen reveal material mirroring the M2.E lifecycle fixture
        // additions: GO3 waits unnoticed in Exile.
        let material = GameObjectId(3);
        before.zones.objects.insert(
            material,
            GameObject {
                id: material,
                physical_card: Some(PhysicalCardId(3)),
                card_definition: CardDefinitionId(3),
                owner: P1,
                controller: P1,
                tapped: false,
                face_down: false,
            },
        );
        before.zones.locations.insert(material, exile_location());
        before.allocators.next_object_id = GameObjectId(4);
        // The M2.E fixture family is decision-free by construction: no
        // preserved request can survive the fixture transition's revision
        // bump, so neither continuation nor request is carried.
        before.execution.pending_decision = None;
        before.execution.continuations.clear();
        validate_engine_state(&before).map_err(HarnessError::StateValidation)?;

        let p1_opaque = before.perspective_identities.players[&P1].next_opaque_object_id;
        let p1_start = before.knowledge.players[&P1].next_visible_sequence.0;
        let p2_opaque = before.perspective_identities.players[&P2].next_opaque_object_id;
        let p2_start = before.knowledge.players[&P2].next_visible_sequence.0;

        let mut transition = FixtureTransition::start(&before).map_err(fixture_transition)?;
        let revealed = transition
            .move_object_incarnation(material, battlefield())
            .map_err(fixture_transition)?;
        // Occurrence 1+2: the public reveal allocates an opaque identity per
        // perspective with explicit-reveal provenance.
        transition
            .apply_occurrence(occurrence(
                P1,
                p1_start,
                IdentityMutationV1::Allocate {
                    opaque: p1_opaque,
                    object: revealed,
                },
                Some(KnowledgeMutationV1::Acquire {
                    opaque: p1_opaque,
                    definition: Some(CardDefinitionId(3)),
                    location: Some(battlefield()),
                    acquisition: observed(
                        p1_start,
                        KnowledgeHistoryChannel::Public,
                        KnowledgeAcquisitionCause::ExplicitReveal,
                    ),
                }),
                PerspectiveObservationPolicyV1::Appeared {
                    from_zone: ZoneKind::Exile,
                    to_zone: ZoneKind::Battlefield,
                    new_object: revealed,
                },
            ))
            .map_err(fixture_transition)?;
        transition
            .apply_occurrence(occurrence(
                P2,
                p2_start,
                IdentityMutationV1::Allocate {
                    opaque: p2_opaque,
                    object: revealed,
                },
                Some(KnowledgeMutationV1::Acquire {
                    opaque: p2_opaque,
                    definition: None,
                    location: Some(battlefield()),
                    acquisition: observed(
                        p2_start,
                        KnowledgeHistoryChannel::Public,
                        KnowledgeAcquisitionCause::ExplicitReveal,
                    ),
                }),
                PerspectiveObservationPolicyV1::Appeared {
                    from_zone: ZoneKind::Exile,
                    to_zone: ZoneKind::Battlefield,
                    new_object: revealed,
                },
            ))
            .map_err(fixture_transition)?;
        // Occurrence 3: tracked hiding remaps P1's opaque identity onto the
        // fresh hidden incarnation and moves the battlefield fact into the
        // ordered history.
        let hidden = transition
            .move_object_incarnation(revealed, hidden_hand(P2))
            .map_err(fixture_transition)?;
        transition
            .apply_occurrence(occurrence(
                P1,
                p1_start + 1,
                IdentityMutationV1::Remap {
                    opaque: p1_opaque,
                    from_object: revealed,
                    to_object: hidden,
                },
                Some(KnowledgeMutationV1::CurrentToHistory {
                    opaque: p1_opaque,
                    observed_definition: Some(CardDefinitionId(3)),
                }),
                PerspectiveObservationPolicyV1::MovedInSight {
                    from_zone: ZoneKind::Battlefield,
                    to_zone: ZoneKind::Hand,
                    old_object: revealed,
                    new_object: hidden,
                    reveals_old: true,
                    reveals_new: false,
                },
            ))
            .map_err(fixture_transition)?;
        // Occurrence 4: a private look re-binds the known location with
        // private-channel PrivateLook provenance.
        transition
            .apply_occurrence(occurrence(
                P1,
                p1_start + 2,
                IdentityMutationV1::None,
                Some(KnowledgeMutationV1::UpdateLocation {
                    opaque: p1_opaque,
                    fact: KnownLocationFactV2 {
                        location: hidden_hand(P2),
                        provenance: observed(
                            p1_start + 2,
                            KnowledgeHistoryChannel::Private,
                            KnowledgeAcquisitionCause::PrivateLook,
                        ),
                    },
                }),
                PerspectiveObservationPolicyV1::NoEnvelope,
            ))
            .map_err(fixture_transition)?;
        // Occurrence 5: deliberate explicit forget retires the mapping and
        // the knowledge record together with public invalidation provenance.
        transition
            .apply_occurrence(occurrence(
                P1,
                p1_start + 3,
                IdentityMutationV1::Retire {
                    opaque: p1_opaque,
                    object: hidden,
                },
                Some(KnowledgeMutationV1::Invalidate {
                    opaque: p1_opaque,
                    reason: KnowledgeInvalidationReason::ExplicitForget,
                    invalidation_provenance: observed(
                        p1_start + 3,
                        KnowledgeHistoryChannel::Public,
                        KnowledgeAcquisitionCause::PublicEvent,
                    ),
                }),
                PerspectiveObservationPolicyV1::NoEnvelope,
            ))
            .map_err(fixture_transition)?;
        // Occurrence 6: the owning perspective remaps its own opaque
        // identity onto the fresh hidden incarnation and privately accounts
        // the hand content through the private OwnPrivateIdentity channel,
        // keeping every actively tracked mapping live.
        transition
            .apply_occurrence(occurrence(
                P2,
                p2_start + 1,
                IdentityMutationV1::Remap {
                    opaque: p2_opaque,
                    from_object: revealed,
                    to_object: hidden,
                },
                Some(KnowledgeMutationV1::UpdateLocation {
                    opaque: p2_opaque,
                    fact: KnownLocationFactV2 {
                        location: hidden_hand(P2),
                        provenance: observed(
                            p2_start + 1,
                            KnowledgeHistoryChannel::Private,
                            KnowledgeAcquisitionCause::OwnPrivateIdentity,
                        ),
                    },
                }),
                PerspectiveObservationPolicyV1::NoEnvelope,
            ))
            .map_err(fixture_transition)?;
        let result = transition.finish().map_err(fixture_transition)?;
        let state = result.next_state;
        validate_engine_state(&state).map_err(HarnessError::StateValidation)?;

        // Composition postconditions: fail closed rather than emit a
        // scenario that silently lost its information richness.
        let knowledge = &state.knowledge.players[&P1];
        let identity = &state.perspective_identities.players[&P1];
        assert_eq!(
            identity.next_opaque_object_id,
            OpaqueObjectId(p1_opaque.0 + 1),
            "P1's opaque allocator must advance beyond its default"
        );
        assert!(identity.retired_object_ids.contains(&p1_opaque));
        assert!(!identity.opaque_to_object.contains_key(&p1_opaque));
        let retired = &knowledge.retired[&p1_opaque];
        assert_eq!(
            retired.invalidation.reason,
            KnowledgeInvalidationReason::ExplicitForget
        );
        let last_known = retired
            .last_known_location
            .as_ref()
            .expect("the retired record keeps its last known location");
        assert!(matches!(
            last_known.provenance,
            KnowledgeAcquisitionReason::Observed {
                channel: KnowledgeHistoryChannel::Private,
                cause: KnowledgeAcquisitionCause::PrivateLook,
                ..
            }
        ));
        assert_eq!(retired.historical_locations.len(), 1);
        assert_eq!(
            knowledge.next_visible_sequence.0,
            p1_start + 4,
            "P1's visible-sequence cursor must advance past its default"
        );
        assert_eq!(
            state.knowledge.players[&P2].next_visible_sequence.0,
            p2_start + 2
        );
        assert_eq!(
            state.perspective_identities.players[&P2].next_opaque_object_id,
            OpaqueObjectId(p2_opaque.0 + 1)
        );
        Ok(state)
    }

    /// Wraps the information-rich state into a validated V3 checkpoint and
    /// accepts it into a live runtime environment (established pipeline).
    pub(crate) fn information_rich_spawned(
    ) -> Result<(TrustedEnvironmentController, [PlayerEndpointHandle; 2]), HarnessError> {
        let state = information_rich_state()?;
        let config = config();
        let wrapped = EnvironmentCheckpointV3::new(
            state.clone(),
            EpisodeStatus::Running,
            EnvironmentLimitCounters::default(),
            config.codec.clone(),
        )
        .map_err(|_| HarnessError::CheckpointInvalid)?;
        wrapped
            .validate()
            .map_err(|_| HarnessError::CheckpointInvalid)?;
        spawn_environment(state, &config)
    }

    pub(crate) fn visible_request(
        handle: &PlayerEndpointHandle,
    ) -> Result<PlayerDecisionRequestV2, HarnessError> {
        handle
            .visible_decision()
            .map_err(|_| HarnessError::EndpointService)?
            .ok_or(HarnessError::TransformPreconditionViolated)
    }

    /// Builds the ChooseNumber answer `value` against a live pending
    /// request; fails closed when the pending domain differs.
    pub(crate) fn choose_count_answer(
        request: &PlayerDecisionRequestV2,
        value: i64,
    ) -> Result<DecisionResponseV2, HarnessError> {
        if !matches!(request.decision, DecisionDomainV2::ChooseNumber { .. }) {
            return Err(HarnessError::TransformPreconditionViolated);
        }
        Ok(DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: request.player_decision_id,
            state_revision: request.state_revision,
            answer: DecisionAnswerV2::ChooseNumber { value },
        })
    }

    /// A well-formed ChooseMany answer carrying a stale player-decision
    /// identity; classification precedes membership validation, so this is
    /// rejected without mutation at any live members stage.
    pub(crate) fn stale_members_answer(
        request: &PlayerDecisionRequestV2,
    ) -> Result<DecisionResponseV2, HarnessError> {
        if !matches!(request.decision, DecisionDomainV2::ChooseMany { .. }) {
            return Err(HarnessError::TransformPreconditionViolated);
        }
        let member = |index: usize| -> Result<mtgml_model::CandidateIdV1, HarnessError> {
            request
                .candidates
                .get(index)
                .map(|candidate| candidate.candidate_id)
                .ok_or(HarnessError::TransformFixtureAbsent)
        };
        Ok(DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: mtgml_model::PlayerDecisionIdV1(
                request
                    .player_decision_id
                    .0
                    .checked_add(1)
                    .ok_or(HarnessError::TransformPreconditionViolated)?,
            ),
            state_revision: request.state_revision,
            answer: DecisionAnswerV2::SelectMany {
                candidate_ids: vec![member(0)?, member(1)?],
            },
        })
    }

    /// Segment-anchor assertion shared by the restore and fork evidence:
    /// the replay segment seeded from `checkpoint` carries exactly the
    /// checkpoint identity fields.
    pub(crate) fn assert_segment_anchor(
        anchor: &InitialEnvironmentIdentityV3,
        checkpoint: &EnvironmentCheckpointV3,
    ) {
        assert_eq!(anchor.state_revision, checkpoint.state.revision);
        assert_eq!(anchor.full_state_digest, checkpoint.state_digest);
        assert_eq!(anchor.episode_status, checkpoint.status);
        assert_eq!(anchor.environment_limit_counters, checkpoint.limit_counters);
        assert_eq!(anchor.checkpoint_codec_identity, checkpoint.codec);
        assert_eq!(anchor.checkpoint_digest, checkpoint.checkpoint_digest);
    }
}

#[cfg(test)]
mod tests {
    use super::support::{
        assert_segment_anchor, choose_count_answer, decision_rich_spawned,
        information_rich_spawned, visible_request, EQUAL_COUNT_VALUE, P1, P2,
    };
    use crate::isolation::fingerprint::{
        assert_fingerprint_policies, capture_complete, capture_snapshot,
        capture_transition_product, FingerprintComparison,
    };
    use crate::isolation::HarnessError;
    use mtgml_decision::DECISION_RESPONSE_V2_SCHEMA;
    use mtgml_environment::{
        CheckpointValidationError, ControllerError, PlayerEndpoint, SyntheticM1EnvironmentBackend,
        TrustedEnvironmentController,
    };
    use mtgml_model::{CandidateIdV1, PlayerDecisionIdV1, StateRevision};
    use mtgml_replay::AuthoritativeReplayV3;
    use mtgml_wire::encode_canonical;

    fn controller_service(_: ControllerError) -> HarnessError {
        HarnessError::ControllerService
    }

    fn bind_failed(_: ControllerError) -> HarnessError {
        HarnessError::BindFailed
    }

    /// A closed-default plausible response referencing nothing foreign; the
    /// only legal input class at a requestless information-rich instant.
    fn plausible_response() -> mtgml_decision::DecisionResponseV2 {
        mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            answer: mtgml_decision::DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(0),
            },
        }
    }

    /// Restore returns the semantic/environment/player groups to the
    /// checkpoint-time bytes, reseeds an empty recorder segment anchored at
    /// the restored identity, and an identical resumed ChooseCount answer
    /// drives the original environment and a twin rebuilt from the same
    /// checkpoint onto a fresh backend to byte-identical products and
    /// successor fingerprints.
    #[test]
    fn restore_decision_rich() -> Result<(), HarnessError> {
        let (controller, endpoints) = decision_rich_spawned()?;
        let fp0 = capture_complete(&controller, &endpoints)?;
        let cp0 = controller.checkpoint().map_err(controller_service)?;

        controller
            .restore(cp0.clone())
            .map_err(controller_service)?;
        let fp_restored = capture_complete(&controller, &endpoints)?;
        assert_fingerprint_policies(
            &fp0,
            &fp_restored,
            FingerprintComparison::ExcludeReplayRecorder,
        )?;

        // Segment anchor: the restored recorder starts an empty segment
        // whose initial identity IS the restored checkpoint identity.
        let exported: AuthoritativeReplayV3 =
            controller.export_replay().map_err(controller_service)?;
        assert!(exported.steps.is_empty());
        assert_segment_anchor(&exported.manifest.initial_identity, &cp0);

        // Old handles observe the cp0-time projections byte-for-byte.
        assert_eq!(capture_snapshot(&endpoints[0])?, fp0.player.p1_snapshot);
        assert_eq!(capture_snapshot(&endpoints[1])?, fp0.player.p2_snapshot);

        // RESUME proof: the identical next input on original and twin.
        let original_request = visible_request(&endpoints[0])?;
        let response = choose_count_answer(&original_request, EQUAL_COUNT_VALUE)?;
        let step_original = endpoints[0]
            .submit(response.clone())
            .map_err(|_| HarnessError::EndpointService)?;
        let product_original = capture_transition_product(Ok(step_original))?;
        assert_eq!(
            product_original.semantic_submission_code.as_deref(),
            Some("accepted")
        );
        let successor_original = capture_complete(&controller, &endpoints)?;

        let backend = SyntheticM1EnvironmentBackend::from_checkpoint(cp0, super::support::config())
            .map_err(|_| HarnessError::SyntheticBackendRejected)?;
        let twin = TrustedEnvironmentController::new(backend);
        let twin_endpoints = [
            twin.bind_player(P1).map_err(bind_failed)?,
            twin.bind_player(P2).map_err(bind_failed)?,
        ];
        assert_eq!(
            capture_snapshot(&twin_endpoints[0])?,
            fp0.player.p1_snapshot
        );
        assert_eq!(
            capture_snapshot(&twin_endpoints[1])?,
            fp0.player.p2_snapshot
        );
        let original_request_bytes =
            encode_canonical(&original_request).map_err(|_| HarnessError::WireEncoding)?;
        let twin_request_bytes = encode_canonical(&visible_request(&twin_endpoints[0])?)
            .map_err(|_| HarnessError::WireEncoding)?;
        assert_eq!(
            twin_request_bytes, original_request_bytes,
            "the restored twin must expose the byte-identical pending request"
        );
        let step_twin = twin_endpoints[0]
            .submit(response)
            .map_err(|_| HarnessError::EndpointService)?;
        let product_twin = capture_transition_product(Ok(step_twin))?;
        assert_eq!(
            product_twin, product_original,
            "byte-identical TransitionVisibleProduct required"
        );
        let successor_twin = capture_complete(&twin, &twin_endpoints)?;
        assert_eq!(
            successor_twin.player, successor_original.player,
            "successor PlayerVisibleFingerprints must match"
        );
        assert_eq!(
            successor_twin.environment, successor_original.environment,
            "EnvironmentFingerprints must match"
        );
        assert_fingerprint_policies(
            &successor_original,
            &successor_twin,
            FingerprintComparison::All,
        )?;
        Ok(())
    }

    /// Same structure over the information-rich scenario. Per-perspective
    /// snapshot byte equality spans the retained-knowledge surfaces: the
    /// canonical `PlayerInformationStateV2` embeds the ascending active-
    /// then-retained record order together with acquisition, known-location
    /// provenance (including private-look channels and sequences), and the
    /// historical-location ordering, so byte equality covers ordering and
    /// provenance without separate accessors. At its requestless instant the
    /// identical resumed input is the closed typed rejection; its product
    /// and every successor group must still match byte-for-byte.
    #[test]
    fn restore_information_rich() -> Result<(), HarnessError> {
        let (controller, endpoints) = information_rich_spawned()?;
        let fp0 = capture_complete(&controller, &endpoints)?;
        // Non-vacuous richness at the visible boundary: both cursors moved
        // past their construction-time defaults and the projections differ.
        assert!(fp0.player.p1_snapshot.current_visible_sequence.0 > 1);
        assert!(fp0.player.p2_snapshot.current_visible_sequence.0 > 1);
        assert_ne!(
            fp0.player.p1_snapshot.information_state_bytes,
            fp0.player.p2_snapshot.information_state_bytes
        );
        let cp0 = controller.checkpoint().map_err(controller_service)?;

        controller
            .restore(cp0.clone())
            .map_err(controller_service)?;
        let fp_restored = capture_complete(&controller, &endpoints)?;
        assert_fingerprint_policies(
            &fp0,
            &fp_restored,
            FingerprintComparison::ExcludeReplayRecorder,
        )?;
        let exported: AuthoritativeReplayV3 =
            controller.export_replay().map_err(controller_service)?;
        assert!(exported.steps.is_empty());
        assert_segment_anchor(&exported.manifest.initial_identity, &cp0);
        assert_eq!(capture_snapshot(&endpoints[0])?, fp0.player.p1_snapshot);
        assert_eq!(capture_snapshot(&endpoints[1])?, fp0.player.p2_snapshot);

        // RESUME proof with the identical (typed-rejected) input.
        let response = plausible_response();
        let step_original = endpoints[0]
            .submit(response.clone())
            .map_err(|_| HarnessError::EndpointService)?;
        let product_original = capture_transition_product(Ok(step_original))?;
        assert_eq!(
            product_original.semantic_submission_code.as_deref(),
            Some("unavailable_decision"),
            "the requestless instant must reject identically on both sides"
        );
        let successor_original = capture_complete(&controller, &endpoints)?;

        let backend = SyntheticM1EnvironmentBackend::from_checkpoint(cp0, super::support::config())
            .map_err(|_| HarnessError::SyntheticBackendRejected)?;
        let twin = TrustedEnvironmentController::new(backend);
        let twin_endpoints = [
            twin.bind_player(P1).map_err(bind_failed)?,
            twin.bind_player(P2).map_err(bind_failed)?,
        ];
        assert_eq!(
            capture_snapshot(&twin_endpoints[0])?,
            fp0.player.p1_snapshot
        );
        assert_eq!(
            capture_snapshot(&twin_endpoints[1])?,
            fp0.player.p2_snapshot
        );
        let step_twin = twin_endpoints[0]
            .submit(response)
            .map_err(|_| HarnessError::EndpointService)?;
        let product_twin = capture_transition_product(Ok(step_twin))?;
        assert_eq!(
            product_twin, product_original,
            "byte-identical TransitionVisibleProduct required"
        );
        let successor_twin = capture_complete(&twin, &twin_endpoints)?;
        assert_eq!(successor_twin.player, successor_original.player);
        assert_eq!(successor_twin.environment, successor_original.environment);
        assert_fingerprint_policies(
            &successor_original,
            &successor_twin,
            FingerprintComparison::All,
        )?;
        Ok(())
    }

    /// A checkpoint whose limit counters were tampered fails closed at the
    /// earliest validation gate (counter tampering breaks checkpoint-digest
    /// consistency before the limit invariant is even reached), and the
    /// failed restore leaves the live COMPLETE fingerprint untouched.
    #[test]
    fn corrupt_checkpoint_restores_fail_closed() -> Result<(), HarnessError> {
        let (controller, endpoints) = decision_rich_spawned()?;
        let cp0 = controller.checkpoint().map_err(controller_service)?;
        let fingerprint_before = capture_complete(&controller, &endpoints)?;

        let mut corrupted = cp0;
        corrupted.limit_counters.accepted_transitions = corrupted
            .limit_counters
            .accepted_transitions
            .checked_add(1)
            .ok_or(HarnessError::TransformPreconditionViolated)?;
        match controller.restore(corrupted) {
            Ok(()) => panic!("a digest-inconsistent checkpoint must fail closed"),
            Err(ControllerError::CheckpointValidation(
                CheckpointValidationError::CheckpointDigest,
            )) => {}
            Err(other) => panic!("closed checkpoint-validation failure required: {other:?}"),
        }

        let fingerprint_after = capture_complete(&controller, &endpoints)?;
        assert_fingerprint_policies(
            &fingerprint_before,
            &fingerprint_after,
            FingerprintComparison::All,
        )?;
        Ok(())
    }
}
