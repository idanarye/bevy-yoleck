use bevy::prelude::*;
use bevy::utils::Uuid;

use crate::exclusive_systems::YoleckExclusiveSystemDirective;
use crate::prelude::*;

pub fn yoleck_map_entity_to_uuid(
    In(entity): In<Option<Entity>>,
    uuid_query: Query<&YoleckEntityUuid>,
) -> Option<Uuid> {
    Some(uuid_query.get(entity?).ok()?.get())
}

pub fn yoleck_exclusive_system_cancellable(
    In(directive): In<YoleckExclusiveSystemDirective>,
    mut ui: ResMut<YoleckUi>,
    keyboard: Res<Input<KeyCode>>,
) -> YoleckExclusiveSystemDirective {
    if matches!(directive, YoleckExclusiveSystemDirective::Finished) {
        return directive;
    }

    if keyboard.just_released(KeyCode::Escape) || ui.button("Abort Entity Selection").clicked() {
        return YoleckExclusiveSystemDirective::Finished;
    }

    directive
}
