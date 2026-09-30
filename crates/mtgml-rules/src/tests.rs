use super::*;

use mtgml_model::{GameObjectId, PlayerId, RuleEventId, StateRevision};
use mtgml_random::RootSeed256;
use mtgml_state::{SyntheticResetInputs, SyntheticV4Setup};

include!("tests/semantic_delta.rs");

mod zone_incarnation_tests {
    use super::*;
    use crate::errors::ZoneIncarnationError;
    include!("tests/zone_incarnation.rs");
}
