use super::*;
use mtgml_decision::PlayerDecisionRequest;
use mtgml_model::{
    CardDefinitionId, DecisionId, EventSequence, FullStateDigestV2, GameObjectId, OpaqueObjectId,
    PhysicalCardId, PlayerId, RuleEventId, StateRevision, ZoneKind,
};
use mtgml_random::{
    RandomStateV1, RandomStreamCursorV1, RandomStreamKeyV1, RandomStreamKindV1,
    RandomValidationError, RootSeed256,
};
use std::collections::BTreeMap;

fn state() -> EngineState {
    let p1 = PlayerId(1);
    let p2 = PlayerId(2);
    let mut players = BTreeMap::new();
    players.insert(
        p1,
        PlayerState {
            life: 40,
            has_lost: false,
        },
    );
    players.insert(
        p2,
        PlayerState {
            life: 40,
            has_lost: false,
        },
    );
    let random = RandomStateV1::default();
    EngineState {
        revision: StateRevision(0),
        core: CoreRulesState {
            players,
            active_player: p1,
            priority_player: p1,
            turn_number: 1,
        },
        zones: ZoneState::default(),
        allocators: IdentityAllocatorState::default(),
        execution: ExecutionState::default(),
        random,
        knowledge: KnowledgeState {
            players: BTreeMap::from([
                (p1, PlayerKnowledgeState::default()),
                (p2, PlayerKnowledgeState::default()),
            ]),
        },
        perspective_identities: PerspectiveIdentityState {
            players: BTreeMap::from([
                (p1, PerspectiveIdentityMap::default()),
                (p2, PerspectiveIdentityMap::default()),
            ]),
        },
        format: FormatState::None,
    }
}

fn add_stream_entry(
    state: &mut EngineState,
    key: RandomStreamKeyV1,
) -> Result<(), RandomValidationError> {
    state
        .random
        .add_stream(key, RandomStreamCursorV1::default())
}

#[test]
fn valid_empty_shell_passes_cross_component_validation() {
    validate_engine_state(&state()).unwrap();
}

#[test]
fn nonempty_ordered_zone_has_a_stable_domain_separated_digest() {
    let mut value = state();
    let object = GameObject {
        id: GameObjectId(1),
        physical_card: Some(PhysicalCardId(1)),
        card_definition: CardDefinitionId(1),
        owner: PlayerId(1),
        controller: PlayerId(1),
        tapped: false,
        face_down: false,
    };
    let location = ZoneLocation {
        zone: ZoneKind::Library,
        player: Some(PlayerId(1)),
        position: ZonePosition::Top { offset: 0 },
        visibility: VisibilityPartition::OwnerOnly,
        partition: None,
    };
    value.zones.objects.insert(object.id, object);
    value
        .zones
        .locations
        .insert(GameObjectId(1), location.clone());
    value
        .zones
        .ordered_zones
        .insert(location.key(), vec![GameObjectId(1)]);
    value.allocators.next_object_id = GameObjectId(2);

    validate_engine_state(&value).unwrap();
    let first = value.digest().unwrap();
    let second = value.digest().unwrap();
    assert_eq!(first, second);
    assert!(String::from_utf8(value.canonical_digest_bytes().unwrap())
        .unwrap()
        .contains("\"ordered_zones\":["));
}

#[test]
fn knowledge_provenance_must_fit_the_perspective_history() {
    let mut value = state();
    let object = GameObject {
        id: GameObjectId(1),
        physical_card: Some(PhysicalCardId(1)),
        card_definition: CardDefinitionId(1),
        owner: PlayerId(1),
        controller: PlayerId(1),
        tapped: false,
        face_down: false,
    };
    let location = ZoneLocation {
        zone: ZoneKind::Hand,
        player: Some(PlayerId(1)),
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::OwnerOnly,
        partition: None,
    };
    value.zones.objects.insert(object.id, object);
    value
        .zones
        .locations
        .insert(GameObjectId(1), location.clone());
    value
        .zones
        .ordered_zones
        .insert(location.key(), vec![GameObjectId(1)]);
    value.allocators.next_object_id = GameObjectId(2);
    value
        .allocators
        .next_opaque_object_id
        .insert(PlayerId(1), OpaqueObjectId(2));
    let identities = value
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identities
        .object_to_opaque
        .insert(GameObjectId(1), OpaqueObjectId(1));
    identities
        .opaque_to_object
        .insert(OpaqueObjectId(1), GameObjectId(1));
    value
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .known_objects
        .insert(
            GameObjectId(1),
            KnownObjectIdentity {
                object: GameObjectId(1),
                physical_card: Some(PhysicalCardId(1)),
                card_definition: Some(CardDefinitionId(1)),
                known_location: Some(location),
                learned_at: KnowledgePoint {
                    channel: KnowledgeHistoryChannel::Private,
                    sequence: EventSequence(1),
                },
                learned_via: KnowledgeAcquisitionReason::OwnZoneIdentity,
            },
        );

    assert_eq!(
        validate_engine_state(&value),
        Err(EngineStateViolation::KnowledgeMismatch)
    );
    value
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .private_history_length = 1;
    validate_engine_state(&value).unwrap();
}

#[test]
fn exact_delta_reapplies_every_component() {
    let before = state();
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.core.turn_number = 2;
    after.allocators.next_rule_event_id = RuleEventId(4);
    after
        .knowledge
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .public_history_length = 7;
    let delta = StateDelta::between(&before, &after, vec![]).unwrap();
    assert_eq!(delta.apply(&before).unwrap(), after);
}

#[test]
fn pending_decision_must_match_state_revision() {
    let mut invalid = state();
    invalid.allocators.next_decision_id = DecisionId(2);
    invalid.execution.pending_decision = Some(PendingDecisionRecord {
        request: PlayerDecisionRequest {
            schema_version: "player-decision-request.v1".into(),
            decision_id: DecisionId(1),
            state_revision: StateRevision(99),
            actor: PlayerId(1),
            visibility: mtgml_decision::DecisionVisibility::Public,
            decision: mtgml_decision::DecisionKind::ChooseOne,
            candidates: vec![],
        },
        candidate_bindings: BTreeMap::new(),
        continuation: None,
    });
    assert_eq!(
        validate_engine_state(&invalid),
        Err(EngineStateViolation::PendingDecisionMismatch)
    );
}

#[test]
fn rng_player_scope_rejects_absent_player() {
    let mut value = state();
    let p3 = PlayerId(3);
    value
        .random
        .add_stream(
            RandomStreamKeyV1::player_scoped(RandomStreamKindV1::SyntheticM1, p3.0),
            RandomStreamCursorV1::default(),
        )
        .unwrap();
    assert_eq!(
        validate_engine_state(&value),
        Err(EngineStateViolation::RandomState)
    );
}

#[test]
fn nonempty_rng_stream_changes_v2_digest() {
    let mut value = state();
    value
        .random
        .add_stream(
            RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
            RandomStreamCursorV1::default(),
        )
        .unwrap();
    validate_engine_state(&value).unwrap();
    let empty_digest = state().digest().unwrap();
    let nonempty_digest = value.digest().unwrap();
    assert_ne!(empty_digest, nonempty_digest);
    let bytes = value.canonical_digest_bytes().unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(
        text.contains("\"random\""),
        "canonical bytes must include random field"
    );
}

#[test]
fn root_seed_change_changes_v2_digest() {
    let seed = RootSeed256::from_lower_hex(&"ab".repeat(32)).unwrap();
    let mut value_a = state();
    let value_b = state();
    value_a.random = RandomStateV1::new(seed);
    validate_engine_state(&value_a).unwrap();
    validate_engine_state(&value_b).unwrap();
    assert_ne!(value_a.digest().unwrap(), value_b.digest().unwrap());
}

#[test]
fn cursor_change_changes_v2_digest() {
    let mut value = state();
    value
        .random
        .add_stream(
            RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
            RandomStreamCursorV1::default(),
        )
        .unwrap();
    validate_engine_state(&value).unwrap();
    let before = value.digest().unwrap();
    value
        .random
        .set_cursor(
            &RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
            RandomStreamCursorV1 { next_raw_u64: 42 },
        )
        .unwrap();
    let after = value.digest().unwrap();
    assert_ne!(before, after);
}

#[test]
fn insertion_order_does_not_change_v2_digest() {
    let mut value_a = state();
    let mut value_b = state();
    let key_global = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
    let key_player = RandomStreamKeyV1::player_scoped(RandomStreamKindV1::SyntheticM1, 1);
    add_stream_entry(&mut value_a, key_global).unwrap();
    add_stream_entry(&mut value_a, key_player).unwrap();
    add_stream_entry(&mut value_b, key_player).unwrap();
    add_stream_entry(&mut value_b, key_global).unwrap();
    validate_engine_state(&value_a).unwrap();
    validate_engine_state(&value_b).unwrap();
    assert_eq!(value_a.digest().unwrap(), value_b.digest().unwrap());
}

#[test]
fn rng_streams_canonical_bytes_sorted_by_key() {
    let mut value = state();
    value
        .random
        .add_stream(
            RandomStreamKeyV1::player_scoped(RandomStreamKindV1::SyntheticM1, 1),
            RandomStreamCursorV1 { next_raw_u64: 7 },
        )
        .unwrap();
    value
        .random
        .add_stream(
            RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
            RandomStreamCursorV1::default(),
        )
        .unwrap();
    validate_engine_state(&value).unwrap();
    let bytes = value.canonical_digest_bytes().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let streams = json["random"]["streams"].as_array().unwrap();
    let keys: Vec<_> = streams
        .iter()
        .map(|s| {
            s["key"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap())
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(
        keys,
        vec![vec![1, 0, 1, 0], vec![1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1]],
        "RNG streams in canonical bytes must be sorted by key"
    );
}

#[test]
fn frozen_empty_state_v2_digest_is_stable() {
    let value = state();
    let digest = value.digest().unwrap();
    assert_eq!(
        digest.as_str(),
        "b25fb0a19adbe75c069e4e58c658ba333ed28ec0a652eb4a93334e65fa35c712",
        "frozen migration evidence: empty state V2 digest must not change"
    );
    let bytes = value.canonical_digest_bytes().unwrap();
    assert_eq!(
        bytes.len(),
        1293,
        "frozen migration evidence: canonical bytes length must not change"
    );
    let expected_canonical = br#"{"allocators":{"next_ability_id":"1","next_continuation_id":"1","next_decision_id":"1","next_effect_id":"1","next_object_id":"1","next_opaque_ability_id":{},"next_opaque_object_id":{},"next_rule_event_id":"1","next_stack_object_id":"1","next_trigger_id":"1"},"core":{"active_player":"1","players":{"1":{"has_lost":false,"life":40},"2":{"has_lost":false,"life":40}},"priority_player":"1","turn_number":1},"domain":"mtgml.full-state-digest.v2","execution":{"continuations":{},"delayed_effects":{},"effects":{},"waiting_triggers":{}},"format":{"kind":"none"},"knowledge":{"players":{"1":{"invalidations":[],"known_objects":{},"private_history_length":0,"public_history_length":0},"2":{"invalidations":[],"known_objects":{},"private_history_length":0,"public_history_length":0}}},"perspective_identities":{"players":{"1":{"ability_to_opaque":{},"object_to_opaque":{},"opaque_to_ability":{},"opaque_to_object":{}},"2":{"ability_to_opaque":{},"object_to_opaque":{},"opaque_to_ability":{},"opaque_to_object":{}}}},"random":{"contract_id":"mtgml.rng.v1","root_seed":"0000000000000000000000000000000000000000000000000000000000000000","streams":[]},"revision":"0","schema_version":"full-state-digest-input.v2","zones":{"locations":{},"objects":{},"ordered_zones":[],"stack_order":[],"stack_records":{}}}"#;
    assert_eq!(
        bytes, expected_canonical,
        "frozen migration evidence: exact canonical bytes must not change"
    );
    let digest_v2 = FullStateDigestV2::from_canonical_bytes(&bytes);
    assert_eq!(
        digest, digest_v2,
        "from_canonical_bytes must reproduce the same frozen digest"
    );
}
