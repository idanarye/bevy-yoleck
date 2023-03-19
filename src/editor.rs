use std::any::TypeId;
use std::sync::Arc;

use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::utils::{HashMap, HashSet};
use bevy_egui::egui;
use serde::Serialize;

use crate::api::YoleckUserSystemContext;
use crate::dynamic_source_handling::YoleckEditingResult;
use crate::{
    BoxedArc, YoleckEditNewStyle, YoleckEditSystems, YoleckEditorEvent, YoleckEditorState,
    YoleckEntityConstructionSpecs, YoleckEntryHeader, YoleckKnobsCache, YoleckManaged,
    YoleckRawEntry, YoleckState, YoleckTypeHandlers, YoleckUi,
};

#[derive(Debug)]
enum YoleckDirectiveInner {
    SetSelected(Option<Entity>),
    PassToEntity(Entity, TypeId, BoxedArc),
    SpawnEntity {
        type_name: String,
        data: serde_json::Value,
        select_created_entity: bool,
    },
}

/// Event that can be sent to control Yoleck's editor.
#[derive(Debug)]
pub struct YoleckDirective(YoleckDirectiveInner);

impl YoleckDirective {
    /// Pass data from an external system (usually a [ViewPort Editing OverLay](crate::vpeol)) to an entity.
    ///
    /// If the entity is currently being edited, this data can be received using the
    /// [`get_passed_data`](crate::YoleckEditContext::get_passed_data) method of
    /// [`YoleckEdit`](crate::YoleckEdit::edit).
    pub fn pass_to_entity<T: 'static + Send + Sync>(entity: Entity, data: T) -> Self {
        Self(YoleckDirectiveInner::PassToEntity(
            entity,
            TypeId::of::<T>(),
            Arc::new(data),
        ))
    }

    /// Set the entity selected in the Yoleck editor.
    pub fn set_selected(entity: Option<Entity>) -> Self {
        Self(YoleckDirectiveInner::SetSelected(entity))
    }

    /// Spawn a new entity with pre-populated data.
    ///
    /// ```no_run
    /// # use serde::{Deserialize, Serialize};
    /// # use bevy::prelude::*;
    /// # use bevy_yoleck::{YoleckEdit, egui, YoleckDirective};
    /// # #[derive(Serialize)]
    /// # struct Example {
    /// #     position: Vec2,
    /// # }
    /// fn duplicate_example(mut edit: YoleckEdit<Example>, mut writer: EventWriter<YoleckDirective>) {
    ///     edit.edit(|_ctx, data, ui| {
    ///         if ui.button("Duplicate").clicked() {
    ///             writer.send(YoleckDirective::spawn_entity(
    ///                 "Example",
    ///                 Example {
    ///                     // Create the new example entity 100 units below the current one:
    ///                     position: data.position - 100.0 * Vec2::Y,
    ///                 },
    ///                 // Automatically select the newly created entity:
    ///                 true,
    ///             ));
    ///         }
    ///     });
    /// }
    /// ```
    pub fn spawn_entity(
        type_name: impl ToString,
        data: impl Serialize,
        select_created_entity: bool,
    ) -> Self {
        Self(YoleckDirectiveInner::SpawnEntity {
            type_name: type_name.to_string(),
            data: serde_json::to_value(data)
                .expect("Serialization of Yoleck values must always work"),
            select_created_entity,
        })
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

/// The UI part for creating new entities. See [`YoleckEditorSections`](crate::YoleckEditorSections).
pub fn new_entity_section(world: &mut World) -> impl FnMut(&mut World, &mut egui::Ui) {
    let mut system_state = SystemState::<(
        Res<YoleckTypeHandlers>,
        Res<State<YoleckEditorState>>,
        EventWriter<YoleckDirective>,
    )>::new(world);

    move |world, ui| {
        let (yoleck_type_handlers, editor_state, mut writer) = system_state.get_mut(world);

        if !matches!(editor_state.0, YoleckEditorState::EditorActive) {
            return;
        }

        let popup_id = ui.make_persistent_id("add_new_entity_popup_id");
        let button_response = ui.button("Add New Entity");
        if button_response.clicked() {
            ui.memory_mut(|memory| memory.toggle_popup(popup_id));
        }

        egui::popup_below_widget(ui, popup_id, &button_response, |ui| {
            for type_name in yoleck_type_handlers.type_handler_names.iter() {
                if ui.button(type_name).clicked() {
                    writer.send(YoleckDirective(YoleckDirectiveInner::SpawnEntity {
                        type_name: type_name.clone(),
                        data: serde_json::Value::Object(Default::default()),
                        select_created_entity: true,
                    }));
                    ui.memory_mut(|memory| memory.toggle_popup(popup_id));
                }
            }
        });

        system_state.apply(world);
    }
}

/// The UI part for selecting entities. See [`YoleckEditorSections`](crate::YoleckEditorSections).
pub fn entity_selection_section(world: &mut World) -> impl FnMut(&mut World, &mut egui::Ui) {
    let mut filter_custom_name = String::new();
    let mut filter_types = HashSet::<String>::new();

    let mut system_state = SystemState::<(
        Res<YoleckState>,
        Res<YoleckTypeHandlers>,
        Query<(Entity, &YoleckManaged)>,
        Res<State<YoleckEditorState>>,
        EventWriter<YoleckDirective>,
    )>::new(world);

    move |world, ui| {
        let (yoleck, yoleck_type_handlers, yoleck_managed_query, editor_state, mut writer) =
            system_state.get_mut(world);

        if !matches!(editor_state.0, YoleckEditorState::EditorActive) {
            return;
        }

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
                        writer.send(YoleckDirective::set_selected(None));
                    } else {
                        writer.send(YoleckDirective::set_selected(Some(entity)));
                    }
                }
            }
        });
    }
}

/// The UI part for editing entities. See [`YoleckEditorSections`](crate::YoleckEditorSections).
pub fn entity_editing_section(world: &mut World) -> impl FnMut(&mut World, &mut egui::Ui) {
    let mut system_state = SystemState::<(
        ResMut<YoleckState>,
        ResMut<YoleckUserSystemContext>,
        Query<(Entity, &mut YoleckManaged)>,
        Query<Entity, With<YoleckEditNewStyle>>,
        EventReader<YoleckDirective>,
        Commands,
        Res<State<YoleckEditorState>>,
        EventWriter<YoleckEditorEvent>,
        ResMut<YoleckKnobsCache>,
        Res<YoleckEntityConstructionSpecs>,
    )>::new(world);

    let mut writer_state = SystemState::<EventWriter<YoleckEditorEvent>>::new(world);

    let mut comparison_cache = None;
    let mut previously_edited_entity: Option<Entity> = None;

    move |world, ui| {
        let mut handler_to_run = None;
        {
            let (
                mut yoleck,
                mut yoleck_user_system_context,
                mut yoleck_managed_query,
                yoleck_edited_query,
                mut directives_reader,
                mut commands,
                editor_state,
                mut writer,
                mut knobs_cache,
                construction_specs,
            ) = system_state.get_mut(world);

            if !matches!(editor_state.0, YoleckEditorState::EditorActive) {
                return;
            }

            let mut data_passed_to_entities: HashMap<Entity, HashMap<TypeId, BoxedArc>> =
                Default::default();
            for directive in directives_reader.iter() {
                match &directive.0 {
                    YoleckDirectiveInner::PassToEntity(entity, type_id, data) => {
                        data_passed_to_entities
                            .entry(*entity)
                            .or_default()
                            .insert(*type_id, data.clone());
                    }
                    YoleckDirectiveInner::SetSelected(entity) => {
                        if let Some(entity) = entity {
                            let mut already_selected = false;
                            for entity_to_deselect in yoleck_edited_query.iter() {
                                if entity_to_deselect == *entity {
                                    already_selected = true;
                                } else {
                                    commands
                                        .entity(entity_to_deselect)
                                        .remove::<YoleckEditNewStyle>();
                                    writer.send(YoleckEditorEvent::EntityDeselected(
                                        entity_to_deselect,
                                    ));
                                }
                            }
                            if !already_selected {
                                commands.entity(*entity).insert(YoleckEditNewStyle {});
                                writer.send(YoleckEditorEvent::EntitySelected(*entity));
                            }
                        } else {
                            for entity_to_deselect in yoleck_edited_query.iter() {
                                commands
                                    .entity(entity_to_deselect)
                                    .remove::<YoleckEditNewStyle>();
                                writer
                                    .send(YoleckEditorEvent::EntityDeselected(entity_to_deselect));
                            }
                        }

                        // TODO: this one can be removed
                        if *entity != yoleck.entity_being_edited {
                            if let Some(entity) = entity {
                                writer.send(YoleckEditorEvent::EntitySelected(*entity));
                            } else {
                                writer.send(YoleckEditorEvent::EntityDeselected(
                                    yoleck
                                        .entity_being_edited
                                        .expect("cannot be None because `entity` is None"),
                                ));
                            }
                            yoleck.entity_being_edited = *entity;
                        }
                    }
                    YoleckDirectiveInner::SpawnEntity {
                        type_name,
                        data,
                        select_created_entity,
                    } => {
                        let Some(component_handlers) = construction_specs.component_handlers_for(type_name) else {
                            error!("Entity type {:?} is not registered", type_name);
                            break;
                        };
                        let mut cmd = commands.spawn(YoleckRawEntry {
                            header: YoleckEntryHeader {
                                type_name: type_name.clone(),
                                name: "".to_owned(),
                            },
                            data: data.clone(),
                        });
                        for handler in component_handlers {
                            (handler.insert_to_command)(&mut cmd, None);
                        }
                        if *select_created_entity {
                            yoleck.entity_being_edited = Some(cmd.id());
                            writer.send(YoleckEditorEvent::EntitySelected(cmd.id()));
                            cmd.insert(YoleckEditNewStyle {});
                            for entity_to_deselect in yoleck_edited_query.iter() {
                                commands
                                    .entity(entity_to_deselect)
                                    .remove::<YoleckEditNewStyle>();
                                writer
                                    .send(YoleckEditorEvent::EntityDeselected(entity_to_deselect));
                            }
                        }
                        yoleck.level_needs_saving = true;
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
                        writer.send(YoleckEditorEvent::EntityDeselected(entity));
                        yoleck.entity_being_edited = None;
                        yoleck.level_needs_saving = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Custom Name:");
                    ui.text_edit_singleline(&mut yoleck_managed.name);
                });

                // `entity_being_edited` will be `None` if we deleted the entity - in which case we
                // don't want to call `on_editor` which will attempt to run more commands on it and
                // panic.
                if yoleck.entity_being_edited.is_some() {
                    handler_to_run = Some(yoleck_managed.type_name.clone());
                    *yoleck_user_system_context = YoleckUserSystemContext::Edit {
                        entity,
                        passed: data_passed_to_entities,
                    };
                }
            }

            if previously_edited_entity != yoleck.entity_being_edited() {
                previously_edited_entity = yoleck.entity_being_edited();
                for knob_entity in knobs_cache.drain() {
                    commands.entity(knob_entity).despawn_recursive();
                }
            } else {
                knobs_cache.clean_untouched(|knob_entity| {
                    commands.entity(knob_entity).despawn_recursive();
                });
            }
        }
        system_state.apply(world);

        let frame = egui::Frame::none();
        let mut prepared = frame.begin(ui);
        let content_ui = std::mem::replace(
            &mut prepared.content_ui,
            ui.child_ui(egui::Rect::EVERYTHING, *ui.layout()),
        );
        world.insert_resource(YoleckUi(content_ui));
        world.resource_scope(|world, mut yoleck_edit_systems: Mut<YoleckEditSystems>| {
            yoleck_edit_systems.run_systems(world);
        });
        let YoleckUi(content_ui) = world
            .remove_resource()
            .expect("The YoleckUi resource was put in the world by this very function");
        prepared.content_ui = content_ui;
        prepared.end(ui);

        // TODO: This part should be removed
        if let Some(type_name) = handler_to_run {
            world.resource_scope(|world, mut yoleck_type_handlers: Mut<YoleckTypeHandlers>| {
                let entity = world
                    .resource::<YoleckUserSystemContext>()
                    .get_edit_entity();
                let handler = yoleck_type_handlers
                    .type_handlers
                    .get_mut(&type_name)
                    .unwrap();
                let edit_result =
                    handler.run_edit_systems(world, ui, entity, &mut comparison_cache);
                if matches!(edit_result, YoleckEditingResult::Changed) {
                    world.resource_mut::<YoleckState>().level_needs_saving = true;
                    *world.resource_mut::<YoleckUserSystemContext>() =
                        YoleckUserSystemContext::PopulateEdited(entity);
                    handler.run_populate_systems(world);
                    let mut writer = writer_state.get_mut(world);
                    writer.send(YoleckEditorEvent::EditedEntityPopulated(entity));
                }
            });
            *world.resource_mut::<YoleckUserSystemContext>() = YoleckUserSystemContext::Nope;
        }
    }
}
