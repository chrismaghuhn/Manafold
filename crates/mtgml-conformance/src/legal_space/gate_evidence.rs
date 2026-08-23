//! Owned exact evidence nodes for the two M2.F gates.

use std::collections::BTreeSet;

use super::canonical::{
    CanonicalCompleteChoice, CanonicalStageChoice, SyntheticChoiceAtom,
};
use super::comparator::{
    completeness_defects, request_sequence_defects, request_shape_mismatches,
    soundness_defects, SpaceDefect,
};
use super::explorer::{
    explore, generate_probes, ExplorationBoundError, ObservedRequest,
    ProductionSpace, ScenarioBindingContext,
};
use super::oracle::ReferenceAutomaton;
use mtgml_decision::{
    CandidateIntent, DecisionDomainV2, DecisionVisibility, PlayerDecisionRequestV2,
    VisibleCandidateV2, PLAYER_DECISION_REQUEST_V2_SCHEMA,
};
use mtgml_environment::{
    EnvironmentCheckpointV3, EnvironmentLimitCounters, SyntheticM1EnvironmentBackend,
    SyntheticM1EnvironmentConfig, SyntheticM1ReplayConfig, TrustedEnvironmentController,
};
use mtgml_model::{
    CheckpointCodecIdentity, CandidateIdV1, ContentDigest, OpaqueObjectId, PlayerId,
    StateRevision,
};
use mtgml_random::RootSeed256;
use mtgml_replay::{DeckIdentityV1, KernelIdentityV1, ReplaySchemaVersionsV1};
use mtgml_state::construct_synthetic_engine_state;

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

fn seed() -> RootSeed256 {
    RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap()
}

fn codec() -> CheckpointCodecIdentity {
    CheckpointCodecIdentity {
        codec_id: "synthetic-m2-memory".into(),
        semantic_version: "3".into(),
    }
}

fn config(players: [PlayerId; 2]) -> SyntheticM1EnvironmentConfig {
    use mtgml_observation::{
        INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA, OBSERVED_EVENT_SCHEMA_V2,
        PLAYER_STEP_SCHEMA_V2,
    };
    SyntheticM1EnvironmentConfig {
        codec: codec(),
        replay: SyntheticM1ReplayConfig {
            engine_build: "synthetic-build".into(),
            kernel: KernelIdentityV1 {
                implementation_id: "synthetic-m2".into(),
                semantic_version: "0.2.2".into(),
                build_profile: "test".into(),
            },
            rules_snapshot: "synthetic-rules".into(),
            format_policy_snapshot: "synthetic-format".into(),
            oracle_snapshot: "synthetic-oracle".into(),
            card_bundle: "synthetic-bundle".into(),
            randomness_contract_id: "mtgml.rng.v1".into(),
            schemas: ReplaySchemaVersionsV1 {
                observation: OBSERVATION_SCHEMA.into(),
                information_state: INFORMATION_STATE_SCHEMA_V2.into(),
                decision: "player-decision-request.v2".into(),
                decision_response: "decision-response.v2".into(),
                observed_event: OBSERVED_EVENT_SCHEMA_V2.into(),
                player_step: PLAYER_STEP_SCHEMA_V2.into(),
                replay_step: "replay-step.v3".into(),
            },
            decks: players
                .into_iter()
                .enumerate()
                .map(|(index, player)| DeckIdentityV1 {
                    player,
                    deck_id: format!("synthetic-deck-{index}"),
                    digest: ContentDigest::from_canonical_bytes(
                        format!("synthetic-deck-{index}").as_bytes(),
                    ),
                })
                .collect(),
        },
    }
}

fn fixture_controller() -> TrustedEnvironmentController {
    let players = [P1, P2];
    let state =
        construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players,
            root_seed: seed(),
        })
        .unwrap();
    let counters = EnvironmentLimitCounters::default();
    let checkpoint = EnvironmentCheckpointV3::new(
        state,
        mtgml_model::EpisodeStatus::Running,
        counters,
        codec(),
    )
    .unwrap();
    TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::from_checkpoint(checkpoint, config(players)).unwrap(),
    )
}

fn context() -> ScenarioBindingContext {
    ScenarioBindingContext {
        entry_anchor_object: OpaqueObjectId(1),
    }
}

fn live_production() -> (Vec<CanonicalCompleteChoice>, ProductionSpace) {
    let controller = fixture_controller();
    let space = explore(&controller, P1, &context(), Default::default()).unwrap();
    let reference = ReferenceAutomaton::initial().enumerate_complete_choices();
    (reference, space)
}

mod soundness {
    use super::*;

    #[test]
    fn live_matrix_passes() {
        let (reference, production) = live_production();
        assert_eq!(reference.len(), 10);
        assert!(soundness_defects(&reference, &production).is_empty());
        assert!(request_shape_mismatches(
            &ReferenceAutomaton::initial(),
            &production
        )
        .is_empty());
    }

    #[test]
    fn detects_wrong_visible_candidate_semantics() {
        // Wrong mode_index must be detected.
        let observed = ObservedRequest {
            domain: super::super::explorer::ObservedDomain::ChooseMany {
                minimum: 2,
                maximum: 2,
            },
            candidate_atoms: vec![SyntheticChoiceAtom::Piece(9)],
        };
        let automaton = ReferenceAutomaton::initial();
        let stages = vec![CanonicalStageChoice::Number(1)];
        let defects =
            request_sequence_defects(&automaton, &stages, &[observed]);
        assert!(defects
            .iter()
            .any(|defect| matches!(defect, SpaceDefect::RequestShapeMismatch { .. })));

        // Wrong entry anchor payload.
        let context = ScenarioBindingContext {
            entry_anchor_object: OpaqueObjectId(999),
        };
        let intent = CandidateIntent::SelectObject {
            object: OpaqueObjectId(1),
        };
        assert!(super::super::explorer::map_candidate(&intent, &context).is_err());
    }

    #[test]
    fn detects_illegal_extra() {
        let (mut reference, mut production) = super::live_production();
        let extra_choice =
            CanonicalCompleteChoice(vec![CanonicalStageChoice::Anchor, CanonicalStageChoice::Number(99)]);
        production.complete_paths.insert(extra_choice.clone(), vec![]);
        let defects = soundness_defects(&reference, &production);
        assert!(defects.iter().any(|defect| matches!(
            defect,
            SpaceDefect::IllegalExtra { choice } if *choice == extra_choice
        )));
        let _ = &mut reference;
    }

    #[test]
    fn detects_advertised_rejected() {
        let (reference, mut production) = super::live_production();
        production.advertised_rejected.push("fabricated".into());
        let defects = soundness_defects(&reference, &production);
        assert!(defects.iter().any(|defect| matches!(
            defect,
            SpaceDefect::AdvertisedRejected { .. }
        )));
    }

    #[test]
    fn detects_numeric_bound_mutants() {
        let (full_reference, production) = super::live_production();
        // Mutated lower bound 1..=3 erases the c=0 branch.
        let mutated: Vec<CanonicalCompleteChoice> = full_reference
            .iter()
            .filter(|choice| {
                !choice.0.iter().any(|stage| {
                    matches!(stage, CanonicalStageChoice::Number(0))
                })
            })
            .cloned()
            .collect();
        let defects = soundness_defects(&mutated, &production);
        assert!(defects.iter().any(|defect| matches!(
            defect,
            SpaceDefect::IllegalExtra { .. }
        )));
    }

    #[test]
    fn detects_cardinality_mutants() {
        let (full_reference, production) = super::live_production();
        // Mutated ChooseMembers minimum=1 erases the valid empty selection.
        let mutated: Vec<CanonicalCompleteChoice> = full_reference
            .iter()
            .filter(|choice| {
                !choice.0.iter().any(|stage| {
                    matches!(
                        stage,
                        CanonicalStageChoice::Members(set) if set.is_empty()
                    )
                })
            })
            .cloned()
            .collect();
        let defects = crate::legal_space::comparator::completeness_defects(&mutated, &production);
        assert!(defects.iter().any(|defect| matches!(
            defect,
            SpaceDefect::MissingChoice { .. }
        )));
    }

    #[test]
    fn detects_illegal_later_stage_choice() {
        let (reference, mut production) = super::live_production();
        let keys: Vec<CanonicalCompleteChoice> = production
            .complete_paths
            .keys()
            .filter(|choice| {
                matches!(
                    choice.0.last(),
                    Some(CanonicalStageChoice::Order(atoms)) if atoms.len() == 3
                )
            })
            .cloned()
            .collect();
        for key in keys {
            let mut stages = key.0.clone();
            stages.push(CanonicalStageChoice::Order(vec![
                SyntheticChoiceAtom::Piece(9),
            ]));
            let illegal = CanonicalCompleteChoice(stages);
            production.complete_paths.insert(illegal.clone(), vec![]);
        }
        let defects = soundness_defects(&reference, &production);
        assert!(defects.iter().any(|defect| matches!(
            defect,
            SpaceDefect::IllegalExtra { .. }
        )));
    }
}

mod unsatisfiable {
    use super::*;

    #[test]
    fn authoritative_request_fails_closed() {
        let request = PlayerDecisionRequestV2 {
            schema_version: PLAYER_DECISION_REQUEST_V2_SCHEMA.into(),
            player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: P1,
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseOne,
            candidates: Vec::new(),
        };
        assert!(request.validate().is_err());

        let request = PlayerDecisionRequestV2 {
            schema_version: PLAYER_DECISION_REQUEST_V2_SCHEMA.into(),
            player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: P1,
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseMany {
                minimum: 5,
                maximum: 5,
            },
            candidates: (0..2u32)
                .map(|index| VisibleCandidateV2 {
                    candidate_id: CandidateIdV1(index),
                    intent: CandidateIntent::SelectMode { mode_index: index },
                })
                .collect(),
        };
        assert!(request.validate().is_err());

        let budget = super::super::explorer::ExplorerBudget::default();
        assert!(generate_probes(&request, &budget).is_err());
    }
}

mod budget {
    use super::*;

    #[test]
    fn violations_fail_closed() {
        let budget = super::super::explorer::ExplorerBudget::default();

        // Numeric span beyond cap.
        let request = PlayerDecisionRequestV2 {
            schema_version: PLAYER_DECISION_REQUEST_V2_SCHEMA.into(),
            player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: P1,
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseNumber {
                minimum: -1000,
                maximum: 1000,
            },
            candidates: Vec::new(),
        };
        assert!(matches!(
            generate_probes(&request, &budget),
            Err(ExplorationBoundError::NumericSpanExceeded)
        ));

        // Candidate count beyond cap.
        let request = PlayerDecisionRequestV2 {
            schema_version: PLAYER_DECISION_REQUEST_V2_SCHEMA.into(),
            player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: P1,
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseMany {
                minimum: 0,
                maximum: 9,
            },
            candidates: (0..9u32)
                .map(|index| VisibleCandidateV2 {
                    candidate_id: CandidateIdV1(index),
                    intent: CandidateIntent::SelectMode { mode_index: index },
                })
                .collect(),
        };
        assert!(matches!(
            generate_probes(&request, &budget),
            Err(ExplorationBoundError::CandidatesExceeded)
        ));
    }
}

mod completeness {
    use super::*;

    #[test]
    fn live_matrix_exactly_once() {
        let (reference, production) = super::live_production();
        
        assert!(completeness_defects(&reference, &production).is_empty());
        let counts: BTreeSet<(i64, usize)> = production
            .complete_paths
            .keys()
            .map(|choice| match &choice.0[1] {
                CanonicalStageChoice::Number(value) => (
                    *value,
                    production.complete_paths[choice].len(),
                ),
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert!(counts.contains(&(0, 1)));
        assert!(counts.contains(&(1, 1)));
        assert!(counts.contains(&(2, 2)));
        assert!(counts.contains(&(3, 6)));
    }

    #[test]
    fn detects_missing_choice() {
        let (reference, production) = super::live_production();
        
        let mut reduced = reference.clone();
        reduced.pop();
        let defects = completeness_defects(&reduced, &production);
        assert!(defects
            .iter()
            .any(|defect| matches!(defect, SpaceDefect::MissingChoice { .. })));
    }

    #[test]
    fn duplicate_paths_are_rejected() {
        let (reference, mut production) = super::live_production();
        for paths in production.complete_paths.values_mut() {
            let clones = paths.clone();
            paths.extend(clones);
        }
        let defects = completeness_defects(&reference, &production);
        assert_eq!(defects.len(), reference.len());
    }

    #[test]
    fn detects_later_stage_omission() {
        let (reference, mut production) = super::live_production();
        let keys: Vec<CanonicalCompleteChoice> = production
            .complete_paths
            .keys()
            .filter(|choice| {
                matches!(
                    choice.0.last(),
                    Some(CanonicalStageChoice::Order(atoms)) if atoms.len() == 3
                )
            })
            .cloned()
            .collect();
        for key in keys {
            production.complete_paths.remove(&key);
        }
        let defects = completeness_defects(&reference, &production);
        assert_eq!(defects.len(), 6);
    }
}

mod invariance {
    use super::*;
    use super::super::oracle::ReferenceAssemblySpec;

    #[test]
    fn set_vs_sequence_mutant_matrix() {
        use SyntheticChoiceAtom::Piece;
        let set_a: BTreeSet<SyntheticChoiceAtom> = [Piece(0), Piece(1)].into();
        let set_b: BTreeSet<SyntheticChoiceAtom> = [Piece(1), Piece(0)].into();
        assert_eq!(
            CanonicalStageChoice::Members(set_a.clone()),
            CanonicalStageChoice::Members(set_b.clone())
        );
        assert_ne!(
            CanonicalStageChoice::Order(vec![Piece(0), Piece(1)]),
            CanonicalStageChoice::Order(vec![Piece(1), Piece(0)])
        );
    }

    #[test]
    fn insertion_order_does_not_change_reference_space() {
        let spec_a = ReferenceAssemblySpec::default();
        let spec_b = ReferenceAssemblySpec {
            piece_iteration_order: vec![2, 1, 0],
        };
        let space_a = ReferenceAutomaton::new(spec_a).enumerate_complete_choices();
        let space_b = ReferenceAutomaton::new(spec_b).enumerate_complete_choices();
        assert_eq!(space_a.len(), space_b.len());
        assert_eq!(space_a, space_b);
    }
}
