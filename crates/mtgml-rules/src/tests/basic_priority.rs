fn priority_state(position: TurnPosition) -> mtgml_state::EngineState {
    let mut state = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(7), PlayerId(42)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup {
            position,
            priority: PriorityState::None,
            combat: None,
            foundation_sources: Default::default(),
        },
    })
    .unwrap();
    state.execution.pending_decision = None;
    state.foundation_sources.insert(
        mtgml_model::GameObjectId(1),
        FoundationCreatureSource {
            source_kind: FoundationSourceKind::Creature,
            base_characteristics: BaseCharacteristics::Simple {
                power: 2,
                toughness: 2,
            },
            marked_damage: 0,
            control_history: ControlHistory::BeforeTurnStart { turn_number: 1 },
        },
    );
    mtgml_state::validate_engine_state(&state).unwrap();
    state
}

fn pass_response(state: &mtgml_state::EngineState) -> DecisionResponseV2 {
    let request = &state.execution.pending_decision.as_ref().unwrap().request;
    DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: request.player_decision_id,
        state_revision: request.state_revision,
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(0),
        },
    }
}

#[test]
fn basic_priority_stable_upkeep_opens_actor_only_single_pass_decision() {
    let before = priority_state(TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    });
    let mut kernel = MagicRulesKernel::s3_b_conformance_candidate();
    let result = kernel.advance_forced_progress(&before).unwrap();
    assert!(result.accepted);
    assert_eq!(result.next_state.revision, StateRevision(1));
    assert_eq!(
        result.next_state.core.priority,
        PriorityState::HeldBy {
            player: PlayerId(7),
            consecutive_passes: 0
        }
    );
    let request = result.next_decision.unwrap();
    assert_eq!(request.actor, PlayerId(7));
    assert_eq!(request.visibility, mtgml_decision::DecisionVisibility::ActingPlayerOnly);
    assert_eq!(request.decision, mtgml_decision::DecisionDomainV2::ChooseOne);
    assert_eq!(request.candidates.len(), 1);
    assert_eq!(request.candidates[0].candidate_id, CandidateIdV1(0));
    assert_eq!(request.candidates[0].visible_intent, mtgml_decision::CandidateIntent::PassPriority);
    assert_eq!(request.candidates[0].trusted_binding, mtgml_decision::EngineCandidateBinding::PassPriority);
    assert!(matches!(
        result.events[0].event,
        crate::AuthoritativeRuleEventKind::PriorityChanged { .. }
    ));
    assert!(matches!(
        result.events[1].event,
        crate::AuthoritativeRuleEventKind::DecisionCreated { .. }
    ));
    assert_eq!(
        result
            .events
            .iter()
            .map(|event| event.event.semantic_delta())
            .collect::<Vec<_>>(),
        result.delta.audit,
        "typed priority audit and semantic delta must match"
    );
}

#[test]
fn basic_priority_first_pass_transfers_and_second_advances_one_step() {
    let before = priority_state(TurnPosition::Ending {
        step: mtgml_state::EndingStep::EndStep,
    });
    let mut kernel = MagicRulesKernel::s3_b_conformance_candidate();
    let opened = kernel.advance_forced_progress(&before).unwrap();
    let first_response = pass_response(&opened.next_state);
    let first = kernel
        .apply(&opened.next_state, PlayerId(7), &first_response)
        .unwrap();
    assert_eq!(first.next_state.revision, StateRevision(2));
    assert_eq!(
        first.next_state.core.priority,
        PriorityState::HeldBy {
            player: PlayerId(42),
            consecutive_passes: 1
        }
    );
    assert_eq!(first.next_decision.as_ref().unwrap().actor, PlayerId(42));
    assert!(matches!(first.events[0].event, crate::AuthoritativeRuleEventKind::DecisionCleared { .. }));
    assert!(matches!(first.events[1].event, crate::AuthoritativeRuleEventKind::PriorityChanged { .. }));
    assert!(matches!(first.events[2].event, crate::AuthoritativeRuleEventKind::DecisionCreated { .. }));

    let second_response = pass_response(&first.next_state);
    let second = kernel
        .apply(&first.next_state, PlayerId(42), &second_response)
        .unwrap();
    assert_eq!(second.next_state.revision, StateRevision(3));
    assert_eq!(second.next_state.core.priority, PriorityState::None);
    assert_eq!(
        second.next_state.core.position,
        TurnPosition::Ending {
            step: mtgml_state::EndingStep::Cleanup
        }
    );
    assert!(second.next_decision.is_none());
    assert!(matches!(second.events[0].event, crate::AuthoritativeRuleEventKind::DecisionCleared { .. }));
    assert!(matches!(second.events[1].event, crate::AuthoritativeRuleEventKind::PriorityChanged { .. }));
    assert!(matches!(second.events[2].event, crate::AuthoritativeRuleEventKind::TurnPositionChanged { .. }));
}

#[test]
fn basic_priority_rejects_wrong_actor_and_stale_response_without_mutation() {
    let state = priority_state(TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    });
    let mut kernel = MagicRulesKernel::s3_b_conformance_candidate();
    let opened = kernel.advance_forced_progress(&state).unwrap();
    let held = opened.next_state;
    let response = pass_response(&held);
    let wrong_actor = kernel.apply(&held, PlayerId(42), &response).unwrap();
    assert!(!wrong_actor.accepted);
    assert_eq!(wrong_actor.next_state, held);
    let mut stale = response;
    stale.state_revision.0 += 1;
    let stale_result = kernel.apply(&held, PlayerId(7), &stale).unwrap();
    assert!(!stale_result.accepted);
    assert_eq!(stale_result.next_state, held);
    let mut wrong_family = pass_response(&held);
    wrong_family.answer = DecisionAnswerV2::Order {
        candidate_ids: vec![CandidateIdV1(0)],
    };
    let wrong_family = kernel.apply(&held, PlayerId(7), &wrong_family).unwrap();
    assert!(!wrong_family.accepted);
    assert_eq!(wrong_family.next_state, held);
}

#[test]
fn basic_priority_terminal_sba_and_unresolved_order_prevent_pass_decisions() {
    let mut losing = priority_state(TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    });
    losing.core.players.get_mut(&PlayerId(7)).unwrap().life = 0;
    let mut kernel = MagicRulesKernel::s3_b_conformance_candidate();
    let terminal = kernel.advance_forced_progress(&losing).unwrap();
    assert!(matches!(terminal.status, mtgml_model::EpisodeStatus::Terminal { .. }));
    assert!(terminal.next_decision.is_none());
    assert_eq!(terminal.next_state.core.priority, PriorityState::None);

    let order_required = two_same_owner_deaths_at_upkeep();
    let ordered = kernel.advance_forced_progress(&order_required).unwrap();
    assert!(ordered.next_decision.is_some());
    assert!(matches!(
        ordered.next_decision.as_ref().unwrap().decision,
        mtgml_decision::DecisionDomainV2::Order { .. }
    ));
    assert_eq!(ordered.next_state.core.priority, PriorityState::None);
}

#[test]
fn basic_priority_identity_exhaustion_is_nonmutating() {
    let mut state = priority_state(TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    });
    state.revision = StateRevision(u64::MAX);
    let before = state.clone();
    let mut kernel = MagicRulesKernel::s3_b_conformance_candidate();
    assert!(kernel.advance_forced_progress(&state).is_err());
    assert_eq!(state, before);

    let exhaustion_mutations: [fn(&mut EngineState); 3] = [
        |state: &mut EngineState| state.allocators.next_decision_id = mtgml_model::DecisionId(u64::MAX),
        |state: &mut EngineState| {
            state
                .perspective_identities
                .players
                .get_mut(&PlayerId(7))
                .unwrap()
                .next_player_decision_id = PlayerDecisionIdV1(u64::MAX)
        },
        |state: &mut EngineState| state.allocators.next_rule_event_id = mtgml_model::RuleEventId(u64::MAX),
    ];
    for mutate in exhaustion_mutations {
        let mut exhausted = priority_state(TurnPosition::Beginning {
            step: BeginningStep::Upkeep,
        });
        mutate(&mut exhausted);
        let before = exhausted.clone();
        let mut kernel = MagicRulesKernel::s3_b_conformance_candidate();
        assert!(kernel.advance_forced_progress(&exhausted).is_err());
        assert_eq!(exhausted, before);
    }
}

#[test]
fn basic_priority_contract_rejects_mismatched_priority_event_before_and_delta() {
    let before = priority_state(TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    });
    let mut kernel = MagicRulesKernel::s3_b_conformance_candidate();
    let opened = kernel.advance_forced_progress(&before).unwrap();
    let mut tampered = opened.clone();
    tampered.events[0].event = crate::AuthoritativeRuleEventKind::PriorityChanged {
        from: PriorityState::HeldBy {
            player: PlayerId(42),
            consecutive_passes: 0,
        },
        to: PriorityState::HeldBy {
            player: PlayerId(7),
            consecutive_passes: 0,
        },
    };
    let audit = tampered
        .events
        .iter()
        .map(|event| event.event.semantic_delta())
        .collect();
    tampered.delta = mtgml_state::StateDelta::between(&before, &tampered.next_state, audit).unwrap();
    assert!(matches!(
        crate::validate_transition_contract(&before, &tampered),
        Err(crate::TransitionViolation::Priority)
    ));
}

#[test]
fn basic_priority_rejects_untap_draw_declaration_damage_and_cleanup_windows() {
    let untap = priority_state(TurnPosition::Beginning {
        step: BeginningStep::Untap,
    });
    let mut untap_kernel = MagicRulesKernel::s3_b_conformance_candidate();
    let untapped = untap_kernel.advance_forced_progress(&untap).unwrap();
    assert!(untapped.next_decision.is_none());
    assert_eq!(untapped.next_state.core.priority, PriorityState::None);
    assert_eq!(
        untapped.next_state.core.position,
        TurnPosition::Beginning {
            step: BeginningStep::Upkeep
        }
    );
    let cleanup = priority_state(TurnPosition::Ending {
        step: mtgml_state::EndingStep::Cleanup,
    });
    let mut cleanup_kernel = MagicRulesKernel::s3_b_conformance_candidate();
    let cleaned = cleanup_kernel.advance_forced_progress(&cleanup).unwrap();
    assert!(cleaned.next_decision.is_none());
    assert_eq!(cleaned.next_state.core.priority, PriorityState::None);
    assert_eq!(
        cleaned.next_state.core.position,
        TurnPosition::Beginning {
            step: BeginningStep::Untap
        }
    );
    let unsupported = [
        TurnPosition::Beginning { step: BeginningStep::Draw },
        TurnPosition::Combat { step: mtgml_state::CombatStep::DeclareAttackers },
        TurnPosition::Combat { step: mtgml_state::CombatStep::DeclareBlockers },
        TurnPosition::Combat { step: mtgml_state::CombatStep::CombatDamage },
        TurnPosition::Combat { step: mtgml_state::CombatStep::BeginningOfCombat },
        TurnPosition::Combat { step: mtgml_state::CombatStep::EndOfCombat },
    ];
    for position in unsupported {
        let state = priority_state(position);
        let mut kernel = MagicRulesKernel::s3_b_conformance_candidate();
        assert!(kernel.advance_forced_progress(&state).is_err(), "{position:?}");
        assert!(state.execution.pending_decision.is_none());
        assert_eq!(state.core.priority, PriorityState::None);
    }
}
