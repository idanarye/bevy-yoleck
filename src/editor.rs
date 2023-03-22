use std::any::TypeId;
use std::sync::Arc;

use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::utils::{HashMap, HashSet};
use bevy_egui::egui;

use crate::entity_management::{YoleckEntryHeader, YoleckRawEntry};
use crate::knobs::{YoleckKnobData, YoleckKnobsCache};
use crate::prelude::{YoleckComponent, YoleckEdit, YoleckUi};
use crate::{
    BoxedArc, YoleckEditSystems, YoleckEntityConstructionSpecs, YoleckManaged, YoleckSchedule,
    YoleckState,
};

/// Whether or not the Yoleck editor is active.
#[derive(States, Default, Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum YoleckEditorState {
    /// Editor mode. The editor is active and can be used to edit entities.
    #[default]
    EditorActive,
    /// Game mode. Either the actual game or playtest from the editor mode.
    GameActive,
}

/// Sync the game's state back and forth when the level editor enters and exits playtest mode.
///
/// Add this as a plugin. When using it, there is no need to initialize the state with `add_state`
/// - `YoleckSyncWithEditorState` will initialize it and set its initial value to `when_editor`.
/// This means that the state's default value should be it's initial value for non-editor mode
/// (which is not necessarily `when_game`, because the game may start in a menu state or a loading
/// state)
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_yoleck::{YoleckSyncWithEditorState, YoleckPluginForEditor, YoleckPluginForGame};
/// # use bevy_yoleck::bevy_egui::EguiPlugin;
/// #[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
/// enum GameState {
///     #[default]
///     Loading,
///     Game,
///     Editor,
/// }
///
/// # let mut app = App::new();
/// # let executable_started_in_editor_mode = true;
/// if executable_started_in_editor_mode {
///     // These two plugins are needed for editor mode:
///     app.add_plugin(EguiPlugin);
///     app.add_plugin(YoleckPluginForEditor);
///
///     app.add_plugin(YoleckSyncWithEditorState {
///         when_editor: GameState::Editor,
///         when_game: GameState::Game,
///     });
/// } else {
///     // This plugin is needed for game mode:
///     app.add_plugin(YoleckPluginForGame);
///
///     app.add_state::<GameState>();
/// }
pub struct YoleckSyncWithEditorState<T>
where
    T: 'static + States + Sync + Send + std::fmt::Debug + Clone + std::cmp::Eq + std::hash::Hash,
{
    pub when_editor: T,
    pub when_game: T,
}

impl<T> Plugin for YoleckSyncWithEditorState<T>
where
    T: 'static + States + Sync + Send + std::fmt::Debug + Clone + std::cmp::Eq + std::hash::Hash,
{
    fn build(&self, app: &mut App) {
        app.add_state::<T>();
        let initial_state = self.when_editor.clone();
        app.add_startup_system(move |mut game_state: ResMut<NextState<T>>| {
            game_state.set(initial_state.clone());
        });
        let when_editor = self.when_editor.clone();
        let when_game = self.when_game.clone();
        app.add_system(
            move |editor_state: Res<State<YoleckEditorState>>,
                  mut game_state: ResMut<NextState<T>>| {
                game_state.set(match editor_state.0 {
                    YoleckEditorState::EditorActive => when_editor.clone(),
                    YoleckEditorState::GameActive => when_game.clone(),
                });
            },
        );
    }
}

/// Events emitted by the Yoleck editor.
///
/// Modules that provide editing overlays over the viewport (like [vpeol](crate::vpeol)) can
/// use these events to update their status to match with the editor.
#[derive(Debug)]
pub enum YoleckEditorEvent {
    EntitySelected(Entity),
    EntityDeselected(Entity),
    EditedEntityPopulated(Entity),
}

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
    /// # use bevy_yoleck::{YoleckEdit, egui, YoleckDirective, YoleckComponent};
    /// # #[derive(Default, Clone, PartialEq, Serialize, Deserialize, Component)]
    /// # struct Example {
    /// #     position: Vec2,
    /// # }
    /// # impl YoleckComponent for Example {
    /// #     const KEY: &'static str = "Example";
    /// # }
    /// fn duplicate_example(mut edit: YoleckEdit<Example>, mut writer: EventWriter<YoleckDirective>) {
    ///     edit.edit(|_ctx, data, ui| {
    ///         if ui.button("Duplicate").clicked() {
    ///             writer.send(
    ///                 YoleckDirective::spawn_entity(
    ///                     "Example",
    ///                     // Automatically select the newly created entity:
    ///                     true,
    ///                 )
    ///                 .with(Example {
    ///                     // Create the new example entity 100 units below the current one:
    ///                     position: data.position - 100.0 * Vec2::Y,
    ///                 })
    ///                 .into(),
    ///             );
    ///         }
    ///     });
    /// }
    /// ```
    pub fn spawn_entity(
        type_name: impl ToString,
        select_created_entity: bool,
    ) -> SpawnEntityBuilder {
        SpawnEntityBuilder {
            type_name: type_name.to_string(),
            select_created_entity,
            data: Default::default(),
        }
    }
}

pub struct SpawnEntityBuilder {
    type_name: String,
    select_created_entity: bool,
    data: HashMap<&'static str, serde_json::Value>,
}

impl SpawnEntityBuilder {
    pub fn with<T: YoleckComponent>(mut self, component: T) -> Self {
        self.data.insert(
            T::KEY,
            serde_json::to_value(component).expect("should always work"),
        );
        self
    }
}

impl From<SpawnEntityBuilder> for YoleckDirective {
    fn from(value: SpawnEntityBuilder) -> Self {
        YoleckDirective(YoleckDirectiveInner::SpawnEntity {
            type_name: value.type_name,
            data: serde_json::to_value(value.data).expect("should always worl"),
            select_created_entity: value.select_created_entity,
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
        Res<YoleckEntityConstructionSpecs>,
        Res<State<YoleckEditorState>>,
        EventWriter<YoleckDirective>,
    )>::new(world);

    move |world, ui| {
        let (construction_specs, editor_state, mut writer) = system_state.get_mut(world);

        if !matches!(editor_state.0, YoleckEditorState::EditorActive) {
            return;
        }

        let popup_id = ui.make_persistent_id("add_new_entity_popup_id");
        let button_response = ui.button("Add New Entity");
        if button_response.clicked() {
            ui.memory_mut(|memory| memory.toggle_popup(popup_id));
        }

        egui::popup_below_widget(ui, popup_id, &button_response, |ui| {
            for entity_type in construction_specs.entity_types.iter() {
                if ui.button(&entity_type.name).clicked() {
                    writer.send(YoleckDirective(YoleckDirectiveInner::SpawnEntity {
                        type_name: entity_type.name.clone(),
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
        Res<YoleckEntityConstructionSpecs>,
        Query<(Entity, &YoleckManaged)>,
        Res<State<YoleckEditorState>>,
        EventWriter<YoleckDirective>,
    )>::new(world);

    move |world, ui| {
        let (yoleck, construction_specs, yoleck_managed_query, editor_state, mut writer) =
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
                for entity_type in construction_specs.entity_types.iter() {
                    let mut should_show = filter_types.contains(&entity_type.name);
                    if ui.checkbox(&mut should_show, &entity_type.name).changed() {
                        if should_show {
                            filter_types.insert(entity_type.name.clone());
                        } else {
                            filter_types.remove(&entity_type.name);
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
        Query<(Entity, &mut YoleckManaged)>,
        Query<Entity, With<YoleckEdit>>,
        EventReader<YoleckDirective>,
        Query<(Entity, Option<&mut YoleckEdit>, Option<&mut YoleckKnobData>)>,
        Commands,
        Res<State<YoleckEditorState>>,
        EventWriter<YoleckEditorEvent>,
        ResMut<YoleckKnobsCache>,
    )>::new(world);

    let mut previously_edited_entity: Option<Entity> = None;

    move |world, ui| {
        {
            let (
                mut yoleck,
                mut yoleck_managed_query,
                yoleck_edited_query,
                mut directives_reader,
                mut data_passing_query,
                mut commands,
                editor_state,
                mut writer,
                mut knobs_cache,
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
                                    commands.entity(entity_to_deselect).remove::<YoleckEdit>();
                                    writer.send(YoleckEditorEvent::EntityDeselected(
                                        entity_to_deselect,
                                    ));
                                }
                            }
                            if !already_selected {
                                commands.entity(*entity).insert(YoleckEdit {
                                    passed_data: Default::default(),
                                });
                                writer.send(YoleckEditorEvent::EntitySelected(*entity));
                            }
                        } else {
                            for entity_to_deselect in yoleck_edited_query.iter() {
                                commands.entity(entity_to_deselect).remove::<YoleckEdit>();
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
                        let mut cmd = commands.spawn(YoleckRawEntry {
                            header: YoleckEntryHeader {
                                type_name: type_name.clone(),
                                name: "".to_owned(),
                            },
                            data: data.clone(),
                        });
                        if *select_created_entity {
                            yoleck.entity_being_edited = Some(cmd.id());
                            writer.send(YoleckEditorEvent::EntitySelected(cmd.id()));
                            cmd.insert(YoleckEdit {
                                passed_data: Default::default(),
                            });
                            for entity_to_deselect in yoleck_edited_query.iter() {
                                commands.entity(entity_to_deselect).remove::<YoleckEdit>();
                                writer
                                    .send(YoleckEditorEvent::EntityDeselected(entity_to_deselect));
                            }
                        }
                        yoleck.level_needs_saving = true;
                    }
                }
            }

            for (entity, edit, knob_data) in data_passing_query.iter_mut() {
                if let Some(mut edit) = edit {
                    edit.passed_data = data_passed_to_entities
                        .get(&entity)
                        .cloned()
                        .unwrap_or_default();
                }
                if let Some(mut knob_data) = knob_data {
                    knob_data.passed_data = data_passed_to_entities
                        .get(&entity)
                        .cloned()
                        .unwrap_or_default();
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

        // Some systems may have edited the entries, so we need to update them
        world.run_schedule(YoleckSchedule::UpdateRawDataFromComponents);
    }
}
