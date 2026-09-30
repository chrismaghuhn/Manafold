//! Unit tests of the production runtime's projections.

use super::*;
use mtgml_model::{EpisodeStatus, PlayerId};
use mtgml_random::RootSeed256;

mod magic_basic_land_observation;
mod successor_turn_projection;

fn seed() -> RootSeed256 {
    RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap()
}
