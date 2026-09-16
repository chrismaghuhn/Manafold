use mtgml_observation::SyntheticM3Observation;
use mtgml_wire::{encode_canonical, WireError};

#[test]
fn p0_m3_observation_uses_the_existing_canonical_json_owner() {
    let encoder: fn(&SyntheticM3Observation) -> Result<Vec<u8>, WireError> =
        encode_canonical::<SyntheticM3Observation>;
    let _ = encoder;
}
