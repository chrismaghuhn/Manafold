use std::mem::size_of;

use mtgml_observation::{SyntheticObservation, SyntheticPriority, SyntheticTurnPosition};

#[test]
fn p0_m3_observation_payload_has_closed_public_types() {
    assert!(size_of::<SyntheticObservation>() > 0);
    assert!(size_of::<SyntheticTurnPosition>() > 0);
    assert!(size_of::<SyntheticPriority>() > 0);
}
