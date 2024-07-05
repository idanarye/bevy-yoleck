use bevy::prelude::*;
use uuid::Uuid;

use crate::exclusive_systems::YoleckExclusiveSystemDirective;
use crate::prelude::*;

/// Transforms an entity to its UUID. Meant to be used with [Yoleck's exclusive edit
/// systems](crate::exclusive_systems::YoleckExclusiveSystemsQueue) and with Bevy's system piping.
///
/// It accepts and returns an `Option` because it is meant to be used with
/// [`vpeol_read_click_on_entity`](crate::vpeol::vpeol_read_click_on_entity).
pub fn yoleck_map_entity_to_uuid(
    In(entity): In<Option<Entity>>,
    uuid_query: Query<&YoleckEntityUuid>,
) -> Option<Uuid> {
    Some(uuid_query.get(entity?).ok()?.get())
}

/// Pipe an [exclusive system](crate::exclusive_systems::YoleckExclusiveSystemsQueue) into this
/// system to make it cancellable by either pressing the Escape key or clicking on a button in the
/// UI.
pub fn yoleck_exclusive_system_cancellable(
    In(directive): In<YoleckExclusiveSystemDirective>,
    mut ui: ResMut<YoleckUi>,
    keyboard: Res<ButtonInput<KeyCode>>,
) -> YoleckExclusiveSystemDirective {
    if matches!(directive, YoleckExclusiveSystemDirective::Finished) {
        return directive;
    }

    if keyboard.just_released(KeyCode::Escape) || ui.button("Abort Entity Selection").clicked() {
        return YoleckExclusiveSystemDirective::Finished;
    }

    directive
}
