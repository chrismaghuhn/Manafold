use mtgml_model::PlayerId;
use mtgml_observation::{
    SyntheticM3BeginningStep, SyntheticM3Observation, SyntheticM3Priority, SyntheticM3TurnPosition,
    SYNTHETIC_M3_OBSERVATION_SCHEMA,
};
use mtgml_wire::{encode_canonical, WireError};

#[test]
fn p0_m3_observation_uses_the_existing_canonical_json_owner() {
    let encoder: fn(&SyntheticM3Observation) -> Result<Vec<u8>, WireError> =
        encode_canonical::<SyntheticM3Observation>;
    let _ = encoder;
}

fn observation(position: SyntheticM3TurnPosition) -> SyntheticM3Observation {
    SyntheticM3Observation {
        schema_version: SYNTHETIC_M3_OBSERVATION_SCHEMA.into(),
        active_player: PlayerId(1),
        turn_number: "1".into(),
        turn_position: position,
        priority: SyntheticM3Priority::None,
    }
}

#[test]
fn p0_m3_observation_untap_payload_has_exact_canonical_bytes() {
    let bytes = encode_canonical(&observation(SyntheticM3TurnPosition::Beginning {
        step: SyntheticM3BeginningStep::Untap,
    }))
    .unwrap();

    assert_eq!(
        bytes,
        br#"{"active_player":"1","priority":{"kind":"none"},"schema_version":"synthetic-m3-observation.v1","turn_number":"1","turn_position":{"kind":"beginning","step":"untap"}}"#
    );
}

#[test]
fn p0_m3_observation_precombat_main_has_no_fake_step_field() {
    let bytes = encode_canonical(&observation(SyntheticM3TurnPosition::PrecombatMain)).unwrap();

    assert_eq!(
        bytes,
        br#"{"active_player":"1","priority":{"kind":"none"},"schema_version":"synthetic-m3-observation.v1","turn_number":"1","turn_position":{"kind":"precombat_main"}}"#
    );
    assert!(!bytes.windows(b"step".len()).any(|window| window == b"step"));
}
