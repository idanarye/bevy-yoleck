use std::any::TypeId;

use bevy::prelude::*;
use bevy::utils::{HashMap, HashSet};
use bevy_egui::{egui, EguiContext};

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

#[allow(clippy::too_many_arguments)]
pub fn yoleck_editor(
    mut egui_context: ResMut<EguiContext>,
    mut yoleck: ResMut<YoleckState>,
    yoleck_type_handlers: Res<YoleckTypeHandlers>,
    mut yoleck_managed_query: Query<(Entity, &mut YoleckManaged)>,
    mut commands: Commands,
    mut filter_custom_name: Local<String>,
    mut filter_types: Local<HashSet<String>>,
    mut directives_reader: EventReader<YoleckDirective>,
) {
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

    egui::Window::new("Level Editor").show(egui_context.ctx_mut(), |ui| {
        let yoleck = yoleck.as_mut();

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

        egui::CollapsingHeader::new("Select").show(ui, |ui| {
            egui::CollapsingHeader::new("Filter").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("By Name:");
                    ui.text_edit_singleline(&mut *filter_custom_name);
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
            for (entity, yoleck_managed) in yoleck_managed_query.iter_mut() {
                if !filter_types.is_empty() && !filter_types.contains(&yoleck_managed.type_name) {
                    continue;
                }
                if !yoleck_managed.name.contains(filter_custom_name.as_str()) {
                    continue;
                }
                let is_selected = yoleck.entity_being_edited == Some(entity);
                if ui
                    .selectable_label(is_selected, format_caption(entity, &yoleck_managed))
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

        if let Some((entity, mut yoleck_managed)) = yoleck
            .entity_being_edited
            .and_then(|entity| yoleck_managed_query.get_mut(entity).ok())
        {
            ui.horizontal(|ui| {
                ui.heading(format!(
                    "Editing {}",
                    format_caption(entity, &yoleck_managed)
                ));
                if ui.button("Delete").clicked() {}
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
            handler.on_editor(
                &mut yoleck_managed.data,
                entity,
                &edit_ctx,
                ui,
                &populate_ctx,
                &mut commands,
            );
        }
    });
}
