use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_egui::egui;

use crate::{YoleckEntryHeader, YoleckManaged, YoleckRawEntry, YoleckTypeHandlers};

// TODO: Make this a proper level files manager
pub fn level_files_manager_section(world: &mut World) -> impl FnMut(&mut World, &mut egui::Ui) {
    let mut system_state =
        SystemState::<(Res<YoleckTypeHandlers>, Query<(Entity, &YoleckManaged)>)>::new(world);
    move |world, ui| {
        let (yoleck_type_handlers, yoleck_managed_query) = system_state.get(world);
        if ui.button("Save Level").clicked() {
            for (entity, yoleck_managed) in yoleck_managed_query.iter() {
                let handler = yoleck_type_handlers
                    .type_handlers
                    .get(&yoleck_managed.type_name)
                    .unwrap();
                println!(
                    "{:?}: {}",
                    entity,
                    serde_json::to_string(&YoleckRawEntry {
                        header: YoleckEntryHeader {
                            type_name: yoleck_managed.type_name.clone(),
                            name: yoleck_managed.name.clone(),
                        },
                        data: handler.make_raw(&yoleck_managed.data),
                    })
                    .unwrap()
                );
            }
        }
    }
}
