fn moved(old_object: Option<u64>, new_object: Option<u64>) -> ObservedEventEnvelopeV2 {
    ObservedEventEnvelopeV2 {
        schema_version: OBSERVED_EVENT_SCHEMA_V2.into(),
        sequence: VisibleSequence(1),
        state_revision: StateRevision(0),
        event: ObservedEventKindV2::ObjectMoved {
            old_object: old_object.map(mtgml_model::OpaqueObjectId),
            new_object: new_object.map(mtgml_model::OpaqueObjectId),
            from: mtgml_model::ZoneKind::Hand,
            to: mtgml_model::ZoneKind::Battlefield,
        },
    }
}

#[test]
fn fnd_015_object_moved_requires_at_least_one_visible_identity() {
    assert!(matches!(
        moved(None, None).validate(),
        Err(ObservationValidationError::ObjectMovedIdentity)
    ));
    assert!(moved(Some(3), None).validate().is_ok());
    assert!(moved(None, Some(11)).validate().is_ok());
    assert!(moved(Some(3), Some(11)).validate().is_ok());
}

fn valid_information_state() -> PlayerInformationStateV2 {
    let state = PlayerInformationStateV2 {
        schema_version: INFORMATION_STATE_SCHEMA_V2.into(),
        perspective: PlayerId(1),
        state_revision: StateRevision(0),
        current_observation: observation(b"{}", b"{}"),
        next_visible_sequence: VisibleSequence(0),
        retained_knowledge: Vec::new(),
        digest: mtgml_model::InformationStateDigestV2::from_canonical_bytes(b"placeholder"),
    };
    state
}

fn valid_current_request() -> mtgml_decision::PlayerDecisionRequestV2 {
    mtgml_decision::PlayerDecisionRequestV2 {
        schema_version: mtgml_decision::PLAYER_DECISION_REQUEST_V2_SCHEMA.into(),
        player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
        state_revision: StateRevision(0),
        actor: PlayerId(1),
        visibility: mtgml_decision::DecisionVisibility::Public,
        decision: mtgml_decision::DecisionDomainV2::ChooseOne,
        candidates: vec![
            mtgml_decision::VisibleCandidateV2 {
                candidate_id: mtgml_model::CandidateIdV1(0),
                intent: mtgml_decision::CandidateIntent::ChooseBoolean { value: false },
            },
            mtgml_decision::VisibleCandidateV2 {
                candidate_id: mtgml_model::CandidateIdV1(1),
                intent: mtgml_decision::CandidateIntent::ChooseBoolean { value: true },
            },
        ],
    }
}

fn rejected_step(
    code: PlayerSubmissionCodeV1,
    next_decision: Option<mtgml_decision::PlayerDecisionRequestV2>,
    status: EpisodeStatus,
) -> PlayerStepV2 {
    PlayerStepV2 {
        schema_version: PLAYER_STEP_SCHEMA_V2.into(),
        information_state: valid_information_state(),
        observed_events: Vec::new(),
        next_decision,
        status,
        submission: PlayerStepSubmissionV1::Rejected { code },
    }
}

#[test]
fn fnd_016a_rejection_matrix_requires_the_correct_decision_presence() {
    let request = valid_current_request();
    for code in [
        PlayerSubmissionCodeV1::StaleDecision,
        PlayerSubmissionCodeV1::InvalidAnswer,
        PlayerSubmissionCodeV1::InvalidCandidate,
        PlayerSubmissionCodeV1::DuplicateAssignment,
        PlayerSubmissionCodeV1::InvalidCardinality,
        PlayerSubmissionCodeV1::InvalidNumber,
        PlayerSubmissionCodeV1::InvalidOrder,
    ] {
        let missing = rejected_step(code, None, EpisodeStatus::Running);
        assert!(missing.validate().is_err());
        let present = rejected_step(
            code,
            Some(request.clone()),
            EpisodeStatus::Running,
        );
        assert!(present.validate().is_ok());
        assert!(present.observed_events.is_empty());
    }

    let unavailable_with_decision = rejected_step(
        PlayerSubmissionCodeV1::UnavailableDecision,
        Some(request),
        EpisodeStatus::Running,
    );
    assert!(unavailable_with_decision.validate().is_err());

    let unavailable_without_decision = rejected_step(
        PlayerSubmissionCodeV1::UnavailableDecision,
        None,
        EpisodeStatus::Running,
    );
    assert!(unavailable_without_decision.validate().is_ok());

    let closed_without_decision = rejected_step(
        PlayerSubmissionCodeV1::EpisodeClosed,
        None,
        EpisodeStatus::Truncated {
            reason: mtgml_model::TruncationReason::ExternalStop,
            players: Vec::new(),
        },
    );
    assert!(closed_without_decision.validate().is_ok());
}
