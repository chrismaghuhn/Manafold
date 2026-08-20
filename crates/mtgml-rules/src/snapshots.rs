use mtgml_model::GameObjectId;
use mtgml_state::{EngineState, ObjectSnapshot};
use std::collections::BTreeMap;

use crate::validation::TransitionViolation;

pub(crate) fn object_snapshots(
    state: &EngineState,
) -> Result<BTreeMap<GameObjectId, ObjectSnapshot>, TransitionViolation> {
    state
        .zones
        .objects
        .iter()
        .map(|(id, object)| {
            let location = state
                .zones
                .locations
                .get(id)
                .ok_or(TransitionViolation::ObjectTraceIncomplete)?;
            Ok((
                *id,
                ObjectSnapshot {
                    object: *id,
                    physical_card: object.physical_card,
                    card_definition: object.card_definition,
                    owner: object.owner,
                    controller: object.controller,
                    tapped: object.tapped,
                    face_down: object.face_down,
                    location: location.clone(),
                },
            ))
        })
        .collect()
}
