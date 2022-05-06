use std::any::TypeId;

use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::utils::{HashMap, HashSet};
use bevy_egui::egui;

use crate::{
    BoxedAny, PopulateReason, YoleckEditContext, YoleckEntryHeader, YoleckManaged,
    YoleckPopulateContext, YoleckRawEntry, YoleckState, YoleckTypeHandlers,
};

pub enum YoleckDirectiveInner {
    SetSelected(Option<Entity>),
    PassToEntity(Entity, TypeId, BoxedAny),
}

pub struct YoleckDirective(YoleckDirectiveInner);

impl YoleckDirective {
    pub fn pass_to_entity<T: 'static + Send + Sync>(entity: Entity, data: T) -> Self {
        Self(YoleckDirectiveInner::PassToEntity(
            entity,
            TypeId::of::<T>(),
            Box::new(data),
        ))
    }

    pub fn set_selected(entity: Option<Entity>) -> Self {
        Self(YoleckDirectiveInner::SetSelected(entity))
    }
}

fn format_caption(entity: Entity, yoleck_managed: &YoleckManaged) -> String {
    if yoleck_managed.name.is_empty() {
        format!("{} {:?}", yoleck_managed.type_name, entity)
    } else {
        format!(
            "{} ({} {:?})",
            yoleck_managed.name, yoleck_managed.type_name, entity
        )
    }
}

pub fn new_entity_section(world: &mut World) -> impl FnMut(&mut World, &mut egui::Ui) {
    let mut system_state =
        SystemState::<(Commands, Res<YoleckTypeHandlers>, ResMut<YoleckState>)>::new(world);

    move |world, ui| {
        let (mut commands, yoleck_type_handlers, mut yoleck) = system_state.get_mut(world);

        let popup_id = ui.make_persistent_id("add_new_entity_popup_id");
        let button_response = ui.button("Add New Entity");
        if button_response.clicked() {
            ui.memory().toggle_popup(popup_id);
        }

        egui::popup_below_widget(ui, popup_id, &button_response, |ui| {
            for type_name in yoleck_type_handlers.type_handler_names.iter() {
                if ui.button(type_name).clicked() {
                    let mut cmd = commands.spawn();
                    cmd.insert(YoleckRawEntry {
                        header: YoleckEntryHeader {
                            type_name: type_name.clone(),
                            name: "".to_owned(),
                        },
                        data: serde_json::Value::Object(Default::default()),
                    });
                    yoleck.entity_being_edited = Some(cmd.id());
                    ui.memory().toggle_popup(popup_id);
                }
            }
        });

        system_state.apply(world);
    }
}

pub fn entity_selection_section(world: &mut World) -> impl FnMut(&mut World, &mut egui::Ui) {
    let mut filter_custom_name = String::new();
    let mut filter_types = HashSet::<String>::new();

    let mut system_state = SystemState::<(
        ResMut<YoleckState>,
        Res<YoleckTypeHandlers>,
        Query<(Entity, &YoleckManaged)>,
    )>::new(world);

    move |world, ui| {
        let (mut yoleck, yoleck_type_handlers, yoleck_managed_query) = system_state.get_mut(world);

        egui::CollapsingHeader::new("Select").show(ui, |ui| {
            egui::CollapsingHeader::new("Filter").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("By Name:");
                    ui.text_edit_singleline(&mut filter_custom_name);
                });
                for type_name in yoleck_type_handlers.type_handler_names.iter() {
                    let mut should_show = filter_types.contains(type_name);
                    if ui.checkbox(&mut should_show, type_name).changed() {
                        if should_show {
                            filter_types.insert(type_name.clone());
                        } else {
                            filter_types.remove(type_name);
                        }
                    }
                }
            });
            for (entity, yoleck_managed) in yoleck_managed_query.iter() {
                if !filter_types.is_empty() && !filter_types.contains(&yoleck_managed.type_name) {
                    continue;
                }
                if !yoleck_managed.name.contains(filter_custom_name.as_str()) {
                    continue;
                }
                let is_selected = yoleck.entity_being_edited == Some(entity);
                if ui
                    .selectable_label(is_selected, format_caption(entity, yoleck_managed))
                    .clicked()
                {
                    if is_selected {
                        yoleck.entity_being_edited = None;
                    } else {
                        yoleck.entity_being_edited = Some(entity);
                    }
                }
            }
        });
    }
}

pub fn entity_editing_section(world: &mut World) -> impl FnMut(&mut World, &mut egui::Ui) {
    let mut system_state = SystemState::<(
        ResMut<YoleckState>,
        Res<YoleckTypeHandlers>,
        Query<(Entity, &mut YoleckManaged)>,
        EventReader<YoleckDirective>,
        Commands,
    )>::new(world);

    move |world, ui| {
        {
            let (
                mut yoleck,
                yoleck_type_handlers,
                mut yoleck_managed_query,
                mut directives_reader,
                mut commands,
            ) = system_state.get_mut(world);

            let mut data_passed_to_entities: HashMap<Entity, HashMap<TypeId, &BoxedAny>> =
                Default::default();
            let dummy_data_passed_to_entity = HashMap::<TypeId, &BoxedAny>::new();
            for directive in directives_reader.iter() {
                match &directive.0 {
                    YoleckDirectiveInner::PassToEntity(entity, type_id, data) => {
                        data_passed_to_entities
                            .entry(*entity)
                            .or_default()
                            .insert(*type_id, data);
                    }
                    YoleckDirectiveInner::SetSelected(entity) => {
                        yoleck.entity_being_edited = *entity;
                    }
                }
            }

            if let Some((entity, mut yoleck_managed)) = yoleck
                .entity_being_edited
                .and_then(|entity| yoleck_managed_query.get_mut(entity).ok())
            {
                ui.horizontal(|ui| {
                    ui.heading(format!(
                        "Editing {}",
                        format_caption(entity, &yoleck_managed)
                    ));
                    if ui.button("Delete").clicked() {
                        commands.entity(entity).despawn_recursive();
                        yoleck.entity_being_edited = None;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Custom Name:");
                    ui.text_edit_singleline(&mut yoleck_managed.name);
                });
                let handler = yoleck_type_handlers
                    .type_handlers
                    .get(&yoleck_managed.type_name)
                    .unwrap();
                let edit_ctx = YoleckEditContext {
                    passed: data_passed_to_entities
                        .get(&entity)
                        .unwrap_or(&dummy_data_passed_to_entity),
                };
                let populate_ctx = YoleckPopulateContext {
                    reason: PopulateReason::EditorUpdate,
                    _phantom_data: Default::default(),
                };
                // `entity_being_edited` will be `None` if we deleted the entity - in which case we
                // don't want to call `on_editor` which will attempt to run more commands on it and
                // panic.
                if yoleck.entity_being_edited.is_some() {
                    handler.on_editor(
                        &mut yoleck_managed.data,
                        entity,
                        &edit_ctx,
                        ui,
                        &populate_ctx,
                        &mut commands,
                    );
                }
            }
        }
        system_state.apply(world);
    }
}
