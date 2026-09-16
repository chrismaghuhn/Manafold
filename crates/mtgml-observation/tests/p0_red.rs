use std::mem::size_of;

use mtgml_observation::{SyntheticM3Observation, SyntheticM3Priority, SyntheticM3TurnPosition};

#[test]
fn p0_m3_observation_payload_has_closed_public_types() {
    assert!(size_of::<SyntheticM3Observation>() > 0);
    assert!(size_of::<SyntheticM3TurnPosition>() > 0);
    assert!(size_of::<SyntheticM3Priority>() > 0);
}
