//! # Your Own Level Editor Creation Kit
//!
//! Yoleck is a crate for having a game built with the Bevy game engine act as its own level
//! editor.
//!
//! Yoleck uses Plain Old Rust Structs to store the data, and uses Serde to store them in files.
//! The user code defines _populate systems_ for creating Bevy entities (populating their
//! components) from these structs and _edit systems_ to edit these structs with egui.
//!
//! The synchronization between the structs and the files is bidirectional, and so is the
//! synchronization between the structs and the egui widgets, but the synchronization from the
//! structs to the entities is unidirectional - changes in the entities are not reflected in the
//! structs:
//!
//! ```none
//! ┌────────┐  Populate   ┏━━━━━━┓   Edit      ┌───────┐
//! │Bevy    │  Systems    ┃Yoleck┃   Systems   │egui   │
//! │Entities│◄────────────┃Struct┃◄═══════════►│Widgets│
//! └────────┘             ┗━━━━━━┛             └───────┘
//!                          ▲
//!                          ║
//!                          ║ Serde
//!                          ║
//!                          ▼
//!                        ┌─────┐
//!                        │.yol │
//!                        │Files│
//!                        └─────┘
//! ```
//!
//! To support integrate Yoleck, a game needs to:
//!
//! * Define the entity structs, and make sure they implement:
//!   ```text
//!   #[derive(Clone, PartialEq, Serialize, Deserialize)]
//!   ```
//!   The structs need to be deserializable form the empty object `{}`, because that's how they'll
//!   be initially created when the editor clicks on _Add New Entity_. Just slap
//!   `#[serde(default)]` on all the fields.
//! * For each struct, use [`add_yoleck_handler`](YoleckExtForApp::add_yoleck_handler) to add a
//!   [`YoleckTypeHandler`] to the Bevy app.
//!   * Register edit systems on the type handler with [`edit_with`](crate::YoleckTypeHandler::edit_with).
//!   * Register populate systems on the type handler with
//!     [`populate_with`](crate::YoleckTypeHandler::populate_with).
//! * If the application starts in editor mode:
//!   * Add the `EguiPlugin` plugin.
//!   * Add the [`YoleckPluginForEditor`] plugin.
//!   * Use [`YoleckSyncWithEditorState`] to synchronize the game's state with the
//!     [`YoleckEditorState`] (optional but highly recommended)
//! * If the application starts in game mode:
//!   * Add the [`YoleckPluginForGame`] plugin.
//!   * Use the [`YoleckLevelIndex`] asset to determine the list of available levels (optional)
//!   * Use [`YoleckLoadingCommand`] to load the level.
//!
//! To support picking and moving entities in the viewport with the mouse, check out the
//! [`vpeol_2d`](crate::vpeol_2d) module. Helpers that can be used in `vpeol_2d` can be found in
//! [`vpeol`](crate::vpeol).
//!
//! # Minimal Working Example
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_yoleck::bevy_egui::EguiPlugin;
//! use bevy_yoleck::{
//!     egui, YoleckEdit, YoleckExtForApp, YoleckLevelIndex, YoleckLoadingCommand,
//!     YoleckPluginForEditor, YoleckPluginForGame, YoleckPopulate, YoleckRawLevel,
//!     YoleckSyncWithEditorState, YoleckTypeHandler,
//! };
//! use serde::{Deserialize, Serialize};
//!
//! fn main() {
//!     let is_editor = std::env::args().any(|arg| arg == "--editor");
//!
//!     let mut app = App::new();
//!     app.add_plugins(DefaultPlugins);
//!     if is_editor {
//!         app.add_plugin(EguiPlugin);
//!         app.add_plugin(YoleckPluginForEditor);
//!         // Doesn't matter in this example, but a proper game would have systems that can work
//!         // on the entity in `GameState::Game`, so while the level is edited we want to be in
//!         // `GameState::Editor` - which can be treated as a pause state. When the editor wants
//!         // to playtest the level we want to move to `GameState::Game` so that they can play it.
//!         app.add_plugin(YoleckSyncWithEditorState {
//!             when_editor: GameState::Editor,
//!             when_game: GameState::Game,
//!         });
//!     } else {
//!         app.add_plugin(YoleckPluginForGame);
//!         app.add_state::<GameState>();
//!         // In editor mode Yoleck takes care of level loading. In game mode the game needs to
//!         // tell yoleck which levels to load and when.
//!         app.add_system(load_first_level.in_set(OnUpdate(GameState::Loading)));
//!     }
//!     app.add_startup_system(setup_camera);
//!
//!     app.add_yoleck_handler({
//!         YoleckTypeHandler::<Rectangle>::new("Rectangle")
//!             .populate_with(populate_rectangle)
//!             .edit_with(edit_rectangle)
//!     });
//!
//!     app.run();
//! }
//!
//! #[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
//! enum GameState {
//!     #[default]
//!     Loading,
//!     Game,
//!     Editor,
//! }
//!
//! fn setup_camera(mut commands: Commands) {
//!     commands.spawn(Camera2dBundle::default());
//! }
//!
//! #[derive(Clone, PartialEq, Serialize, Deserialize)]
//! struct Rectangle {
//!     #[serde(default = "default_rectangle_side")]
//!     width: f32,
//!     #[serde(default = "default_rectangle_side")]
//!     height: f32,
//! }
//!
//! fn default_rectangle_side() -> f32 {
//!     50.0
//! }
//!
//! fn populate_rectangle(mut populate: YoleckPopulate<Rectangle>) {
//!     populate.populate(|_ctx, data, mut cmd| {
//!         cmd.insert(SpriteBundle {
//!             sprite: Sprite {
//!                 color: Color::RED,
//!                 custom_size: Some(Vec2::new(data.width, data.height)),
//!                 ..Default::default()
//!             },
//!             ..Default::default()
//!         });
//!     });
//! }
//!
//! fn edit_rectangle(mut edit: YoleckEdit<Rectangle>) {
//!     edit.edit(|_ctx, data, ui| {
//!         ui.add(egui::Slider::new(&mut data.width, 50.0..=500.0).prefix("Width: "));
//!         ui.add(egui::Slider::new(&mut data.height, 50.0..=500.0).prefix("Height: "));
//!     });
//! }
//!
//! fn load_first_level(
//!     asset_server: Res<AssetServer>,
//!     level_index_assets: Res<Assets<YoleckLevelIndex>>,
//!     mut loading_command: ResMut<YoleckLoadingCommand>,
//!     mut game_state: ResMut<NextState<GameState>>,
//! ) {
//!     let level_index_handle: Handle<YoleckLevelIndex> = asset_server.load("levels/index.yoli");
//!     if let Some(level_index) = level_index_assets.get(&level_index_handle) {
//!         // A proper game would have a proper level progression system, but here we are just
//!         // taking the first level and loading it.
//!         let level_handle: Handle<YoleckRawLevel> =
//!             asset_server.load(&format!("levels/{}", level_index[0].filename));
//!         *loading_command = YoleckLoadingCommand::FromAsset(level_handle);
//!         game_state.set(GameState::Game);
//!     }
//! }
//! ```

mod api;
mod editor;
mod editor_window;
mod entity_management;
mod entity_upgrading;
mod knobs;
mod level_files_manager;
pub mod level_files_upgrading;
mod level_index;
#[cfg(feature = "vpeol")]
pub mod vpeol;
#[cfg(feature = "vpeol_2d")]
pub mod vpeol_2d;

use std::any::{Any, TypeId};
use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;

use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy::utils::HashMap;

pub use self::api::{
    YoleckComponent, YoleckEdit, YoleckEditorEvent, YoleckEditorState, YoleckEntityType,
    YoleckKnobHandle, YoleckKnobs, YoleckPopulate, YoleckPopulateContext,
    YoleckSyncWithEditorState, YoleckUi,
};
pub use self::editor::YoleckDirective;
pub use self::editor_window::YoleckEditorSection;
use self::entity_management::EntitiesToPopulate;
pub use self::entity_management::{
    YoleckEntryHeader, YoleckLoadingCommand, YoleckRawEntry, YoleckRawLevel,
};
use self::entity_upgrading::YoleckEntityUpgrading;
pub use self::entity_upgrading::YoleckEntityUpgradingPlugin;
pub use self::knobs::{YoleckKnobData, YoleckKnobsCache};
pub use self::level_files_manager::YoleckEditorLevelsDirectoryPath;
pub use self::level_index::{YoleckLevelIndex, YoleckLevelIndexEntry};
pub use bevy_egui;
pub use bevy_egui::egui;

struct YoleckPluginBase;
pub struct YoleckPluginForGame;
pub struct YoleckPluginForEditor;

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
enum YoleckSystemSet {
    ProcessRawEntities,
    RunPopulateSchedule,
}

impl Plugin for YoleckPluginBase {
    fn build(&self, app: &mut App) {
        app.insert_resource(YoleckLoadingCommand::NoCommand);
        app.add_asset::<YoleckRawLevel>();
        app.add_asset_loader(entity_management::YoleckLevelAssetLoader);
        app.add_asset::<YoleckLevelIndex>();
        app.add_asset_loader(level_index::YoleckLevelIndexLoader);

        app.configure_sets(
            (
                YoleckSystemSet::ProcessRawEntities,
                YoleckSystemSet::RunPopulateSchedule,
            )
                .chain(),
        );

        app.add_system(
            entity_management::yoleck_process_raw_entries
                .in_set(YoleckSystemSet::ProcessRawEntities),
        );
        app.insert_resource(EntitiesToPopulate(Default::default()));
        app.add_systems(
            (
                entity_management::yoleck_prepare_populate_schedule,
                entity_management::yoleck_run_populate_schedule.run_if(
                    |entities_to_populate: Res<EntitiesToPopulate>| {
                        !entities_to_populate.0.is_empty()
                    },
                ),
            )
                .chain()
                .in_set(YoleckSystemSet::RunPopulateSchedule),
        );
        app.add_system(entity_management::yoleck_process_loading_command);
        app.add_schedule(YoleckSchedule::Populate, {
            let mut schedule = Schedule::new();
            schedule.set_default_base_set(YoleckPopulateBaseSet::RunPopulateSystems);
            schedule
        });
    }
}

impl Plugin for YoleckPluginForGame {
    fn build(&self, app: &mut App) {
        app.add_state::<YoleckEditorState>();
        app.add_startup_system(|mut state: ResMut<NextState<YoleckEditorState>>| {
            state.set(YoleckEditorState::GameActive);
        });
        app.add_plugin(YoleckPluginBase);
    }
}

impl Plugin for YoleckPluginForEditor {
    fn build(&self, app: &mut App) {
        app.add_state::<YoleckEditorState>();
        app.add_event::<YoleckEditorEvent>();
        app.add_plugin(YoleckPluginBase);
        app.insert_resource(YoleckKnobsCache::default());
        app.insert_resource(YoleckState {
            entity_being_edited: None,
            level_needs_saving: false,
        });
        app.insert_resource(YoleckEditorLevelsDirectoryPath(
            Path::new(".").join("assets").join("levels"),
        ));
        app.insert_resource(YoleckEditorSections::default());
        app.add_event::<YoleckDirective>();
        app.add_system(
            editor_window::yoleck_editor_window.after(YoleckSystemSet::ProcessRawEntities),
        );

        app.add_schedule(YoleckSchedule::UpdateRawDataFromComponents, Schedule::new());
    }
}

pub trait YoleckExtForApp {
    /// TODO: document
    fn add_yoleck_edit_system<P>(&mut self, system: impl IntoSystem<(), (), P>);
    fn add_yoleck_entity_type(&mut self, entity_type: YoleckEntityType);
    fn yoleck_populate_schedule_mut(&mut self) -> &mut Schedule;

    fn add_yoleck_entity_upgrade(
        &mut self,
        to_version: usize,
        upgrade_dlg: impl 'static + Send + Sync + Fn(&str, &mut serde_json::Value),
    );

    fn add_yoleck_entity_upgrade_for(
        &mut self,
        to_version: usize,
        for_type_name: impl ToString,
        upgrade_dlg: impl 'static + Send + Sync + Fn(&mut serde_json::Value),
    ) {
        let for_type_name = for_type_name.to_string();
        self.add_yoleck_entity_upgrade(to_version, move |type_name, data| {
            if type_name == for_type_name {
                upgrade_dlg(data);
            }
        });
    }
}

impl YoleckExtForApp for App {
    fn add_yoleck_edit_system<P>(&mut self, system: impl IntoSystem<(), (), P>) {
        let mut system = IntoSystem::into_system(system);
        system.initialize(&mut self.world);
        let mut edit_systems = self
            .world
            .get_resource_or_insert_with(YoleckEditSystems::default);
        edit_systems.edit_systems.push(Box::new(system));
    }

    fn add_yoleck_entity_type(&mut self, entity_type: YoleckEntityType) {
        let construction_specs = self
            .world
            .get_resource_or_insert_with(YoleckEntityConstructionSpecs::default);

        let mut component_type_ids = Vec::with_capacity(entity_type.components.len());
        let mut component_handlers_to_register = Vec::new();
        for handler in entity_type.components.into_iter() {
            component_type_ids.push(handler.component_type());
            if !construction_specs
                .component_handlers
                .contains_key(&handler.component_type())
            {
                component_handlers_to_register.push(handler);
            }
        }

        for handler in component_handlers_to_register.iter() {
            handler.build_in_bevy_app(self);
        }

        let new_entry = YoleckEntityTypeInfo {
            name: entity_type.name.clone(),
            components: component_type_ids,
            on_init: entity_type.on_init,
        };

        let mut construction_specs = self
            .world
            .get_resource_mut::<YoleckEntityConstructionSpecs>()
            .expect("YoleckEntityConstructionSpecs was inserted earlier in this function");

        let new_index = construction_specs.entity_types.len();
        construction_specs
            .entity_types_index
            .insert(entity_type.name, new_index);
        construction_specs.entity_types.push(new_entry);
        for handler in component_handlers_to_register {
            // Can handlers can register systems? If so, this needs to be broken into two phases...
            construction_specs
                .component_handlers
                .insert(handler.component_type(), handler);
        }
    }

    fn yoleck_populate_schedule_mut(&mut self) -> &mut Schedule {
        self
            .get_schedule_mut(YoleckSchedule::Populate)
            .expect("Yoleck's populate schedule was not created. Please use a YoleckPluginForGame or YoleckPluginForEditor")
    }

    fn add_yoleck_entity_upgrade(
        &mut self,
        to_version: usize,
        upgrade_dlg: impl 'static + Send + Sync + Fn(&str, &mut serde_json::Value),
    ) {
        let mut entity_upgrading = self.world.get_resource_mut::<YoleckEntityUpgrading>()
            .expect("add_yoleck_entity_upgrade can only be called after the YoleckEntityUpgrading plugin was added");
        if entity_upgrading.app_format_version < to_version {
            panic!("Cannot create an upgrade system to version {} when YoleckEntityUpgrading set the version to {}", to_version, entity_upgrading.app_format_version);
        }
        entity_upgrading
            .upgrade_functions
            .entry(to_version)
            .or_default()
            .push(Box::new(upgrade_dlg));
    }
}

type BoxedArc = Arc<dyn Send + Sync + Any>;
type BoxedAny = Box<dyn Send + Sync + Any>;

/// A component that describes how Yoleck manages an entity under its control.
#[derive(Component)]
pub struct YoleckManaged {
    /// A name to display near the entity in the entities list.
    ///
    /// This is for level editors' convenience only - it will not be used in the games.
    pub name: String,
    /// This is the name passed to [`YoleckTypeHandler`](YoleckTypeHandler::new).
    pub type_name: String,

    pub needs_to_be_populated: bool,

    pub components_data: HashMap<TypeId, BoxedAny>,
}

#[derive(Default, Resource)]
struct YoleckEditSystems {
    edit_systems: Vec<Box<dyn System<In = (), Out = ()>>>,
}

impl YoleckEditSystems {
    pub(crate) fn run_systems(&mut self, world: &mut World) {
        for system in self.edit_systems.iter_mut() {
            system.run((), world);
            system.apply_buffers(world);
        }
    }
}

pub struct YoleckEntityTypeInfo {
    pub name: String,
    pub components: Vec<TypeId>,
    #[allow(clippy::type_complexity)]
    pub(crate) on_init:
        Vec<Box<dyn 'static + Sync + Send + Fn(YoleckEditorState, &mut EntityCommands)>>,
}

#[derive(Default, Resource)]
pub(crate) struct YoleckEntityConstructionSpecs {
    pub entity_types: Vec<YoleckEntityTypeInfo>,
    pub entity_types_index: HashMap<String, usize>,
    pub component_handlers: HashMap<TypeId, Box<dyn YoleckComponentHandler>>,
}

impl YoleckEntityConstructionSpecs {
    pub fn get_entity_type_info(&self, entity_type: &str) -> Option<&YoleckEntityTypeInfo> {
        Some(&self.entity_types[*self.entity_types_index.get(entity_type)?])
    }
}

/// Fields of the Yoleck editor.
#[derive(Resource)]
pub struct YoleckState {
    entity_being_edited: Option<Entity>,
    level_needs_saving: bool,
}

impl YoleckState {
    pub fn entity_being_edited(&self) -> Option<Entity> {
        self.entity_being_edited
    }
}

/// Sections for the Yoleck editor window.
///
/// Already contains sections by default, but can be used to customize the editor by adding more
/// sections. Each section is a function/closure that accepts a world and returns a closure that
/// accepts as world and a UI. The outer closure is responsible for prepareing a `SystemState` for
/// the inner closure to use.
///
/// ```no_run
/// # use bevy::prelude::*;
/// use bevy::ecs::system::SystemState;
/// # use bevy_yoleck::{YoleckEditorSections, egui};
/// # let mut app = App::new();
/// app.world.resource_mut::<YoleckEditorSections>().0.push((|world: &mut World| {
///     let mut system_state = SystemState::<(
///         Res<Time>,
///     )>::new(world);
///     move |world: &mut World, ui: &mut egui::Ui| {
///         let (
///             time,
///         ) = system_state.get_mut(world);
///         ui.label(format!("Time since startup is {:?}", time.elapsed()));
///     }
/// }).into());
/// ```
#[derive(Resource)]
pub struct YoleckEditorSections(pub Vec<YoleckEditorSection>);

impl Default for YoleckEditorSections {
    fn default() -> Self {
        YoleckEditorSections(vec![
            level_files_manager::level_files_manager_section.into(),
            editor::new_entity_section.into(),
            editor::entity_selection_section.into(),
            editor::entity_editing_section.into(),
        ])
    }
}

trait YoleckComponentHandler: 'static + Sync + Send {
    fn component_type(&self) -> TypeId;
    fn key(&self) -> &'static str;
    fn insert_to_command(&self, cmd: &mut EntityCommands, data: Option<serde_json::Value>);
    fn build_in_bevy_app(&self, app: &mut App);
    fn parse(&self, data: serde_json::Value) -> serde_json::Result<BoxedAny>;
    fn serialize(&self, component: &dyn Any) -> serde_json::Value;
}

#[derive(Default)]
struct YoleckComponentHandlerImpl<T: YoleckComponent> {
    _phantom_data: PhantomData<T>,
}

impl<T: YoleckComponent> YoleckComponentHandler for YoleckComponentHandlerImpl<T> {
    fn component_type(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn key(&self) -> &'static str {
        T::KEY
    }

    fn insert_to_command(&self, cmd: &mut EntityCommands, data: Option<serde_json::Value>) {
        let component: T = if let Some(data) = data {
            match serde_json::from_value(data) {
                Ok(component) => component,
                Err(err) => {
                    error!("Cannot load {:?}: {:?}", T::KEY, err);
                    return;
                }
            }
        } else {
            Default::default()
        };
        cmd.insert(component);
    }

    fn build_in_bevy_app(&self, app: &mut App) {
        if let Some(schedule) = app.get_schedule_mut(YoleckSchedule::UpdateRawDataFromComponents) {
            schedule.add_system(|mut query: Query<(&mut YoleckManaged, &mut T)>| {
                for (mut yoleck_managed, component) in query.iter_mut() {
                    let yoleck_managed = yoleck_managed.as_mut();
                    match yoleck_managed.components_data.entry(TypeId::of::<T>()) {
                        bevy::utils::hashbrown::hash_map::Entry::Vacant(entry) => {
                            yoleck_managed.needs_to_be_populated = true;
                            entry.insert(Box::<T>::new(component.clone()));
                        }
                        bevy::utils::hashbrown::hash_map::Entry::Occupied(mut entry) => {
                            let existing: &T = entry
                                .get()
                                .downcast_ref()
                                .expect("Component data is of wrong type");
                            if existing != component.as_ref() {
                                yoleck_managed.needs_to_be_populated = true;
                                entry.insert(Box::<T>::new(component.as_ref().clone()));
                            }
                        }
                    }
                }
            });
        }
    }

    fn parse(&self, data: serde_json::Value) -> serde_json::Result<BoxedAny> {
        let component: T = serde_json::from_value(data)?;
        Ok(Box::new(component))
    }

    fn serialize(&self, component: &dyn Any) -> serde_json::Value {
        let concrete = component
            .downcast_ref::<T>()
            .expect("Serialize must be called with the correct type");
        serde_json::to_value(concrete).expect("Component must always be serializable")
    }
}

#[derive(ScheduleLabel, Clone, PartialEq, Eq, Debug, Hash)]
pub(crate) enum YoleckSchedule {
    UpdateRawDataFromComponents,
    Populate,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
#[system_set(base)]
pub enum YoleckPopulateBaseSet {
    RunPopulateSystems,
    AddTransform,
}
