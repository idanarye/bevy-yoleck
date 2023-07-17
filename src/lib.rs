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
//! ┌────────┐  Populate   ┏━━━━━━━━━┓   Edit      ┌───────┐
//! │Bevy    │  Systems    ┃Yoleck   ┃   Systems   │egui   │
//! │Entities│◄────────────┃Component┃◄═══════════►│Widgets│
//! └────────┘             ┃Structs  ┃             └───────┘
//!                        ┗━━━━━━━━━┛
//!                            ▲
//!                            ║
//!                            ║ Serde
//!                            ║
//!                            ▼
//!                          ┌─────┐
//!                          │.yol │
//!                          │Files│
//!                          └─────┘
//! ```
//!
//! To support integrate Yoleck, a game needs to:
//!
//! * Define the component structs, and make sure they implement:
//!   ```text
//!   #[derive(Default, Clone, PartialEq, Component, Serialize, Deserialize, YoleckComponent)]
//!   ```
//! * For each entity type that can be created in the level editor, use
//!   [`add_yoleck_entity_type`](YoleckExtForApp::add_yoleck_entity_type) to add a
//!   [`YoleckEntityType`]. Use [`YoleckEntityType::with`] to register the
//!   [`YoleckComponent`](crate::specs_registration::YoleckComponent)s for that entity type.
//! * Register edit systems with
//!   [`add_yoleck_edit_system`](YoleckExtForApp::add_yoleck_edit_system).
//! * Register populate systems on [the populate
//!   schedule](YoleckExtForApp::yoleck_populate_schedule_mut)
//! * If the application starts in editor mode:
//!   * Add the `EguiPlugin` plugin.
//!   * Add the [`YoleckPluginForEditor`] plugin.
//!   * Use [`YoleckSyncWithEditorState`](crate::editor::YoleckSyncWithEditorState) to synchronize
//!     the game's state with the [`YoleckEditorState`] (optional but highly recommended)
//! * If the application starts in game mode:
//!   * Add the [`YoleckPluginForGame`] plugin.
//!   * Use the [`YoleckLevelIndex`] asset to determine the list of available levels (optional)
//!   * Use [`YoleckLoadingCommand`] to load the level.
//!
//! To support picking and moving entities in the viewport with the mouse, check out the
//! [`vpeol_2d`](crate::vpeol_2d) and [`vpeol_3d`](crate::vpeol_3d) modules. After adding the
//! appropriate feature flag (`vpeol_2d`/`vpeol_3d`), import their types from
//! [`bevy_yoleck::vpeol::prelude::*`](crate::vpeol::prelude).
//!
//! # Example
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_yoleck::bevy_egui::EguiPlugin;
//! use bevy_yoleck::prelude::*;
//! use serde::{Deserialize, Serialize};
//! # use bevy_yoleck::egui;
//!
//! fn main() {
//!     let is_editor = std::env::args().any(|arg| arg == "--editor");
//!
//!     let mut app = App::new();
//!     app.add_plugins(DefaultPlugins);
//!     if is_editor {
//!         // Doesn't matter in this example, but a proper game would have systems that can work
//!         // on the entity in `GameState::Game`, so while the level is edited we want to be in
//!         // `GameState::Editor` - which can be treated as a pause state. When the editor wants
//!         // to playtest the level we want to move to `GameState::Game` so that they can play it.
//!         app.add_plugins((
//!             YoleckSyncWithEditorState {
//!                 when_editor: GameState::Editor,
//!                 when_game: GameState::Game,
//!             },
//!             EguiPlugin,
//!             YoleckPluginForEditor
//!         ));
//!     } else {
//!         app.add_plugins(YoleckPluginForGame);
//!         app.add_state::<GameState>();
//!         // In editor mode Yoleck takes care of level loading. In game mode the game needs to
//!         // tell yoleck which levels to load and when.
//!         app.add_systems(Update, load_first_level.run_if(in_state(GameState::Loading)));
//!     }
//!     app.add_systems(Startup, setup_camera);
//!
//!     app.add_yoleck_entity_type({
//!         YoleckEntityType::new("Rectangle")
//!             .with::<Rectangle>()
//!     });
//!     app.add_yoleck_edit_system(edit_rectangle);
//!     app.yoleck_populate_schedule_mut().add_systems(populate_rectangle);
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
//! #[derive(Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
//! struct Rectangle {
//!     width: f32,
//!     height: f32,
//! }
//!
//! impl Default for Rectangle {
//!     fn default() -> Self {
//!         Self {
//!             width: 50.0,
//!             height: 50.0,
//!         }
//!     }
//! }
//!
//! fn populate_rectangle(mut populate: YoleckPopulate<&Rectangle>) {
//!     populate.populate(|_ctx, mut cmd, rectangle| {
//!         cmd.insert(SpriteBundle {
//!             sprite: Sprite {
//!                 color: Color::RED,
//!                 custom_size: Some(Vec2::new(rectangle.width, rectangle.height)),
//!                 ..Default::default()
//!             },
//!             ..Default::default()
//!         });
//!     });
//! }
//!
//! fn edit_rectangle(mut ui: NonSendMut<YoleckUi>, mut edit: YoleckEdit<&mut Rectangle>) {
//!     let Ok(mut rectangle) = edit.get_single_mut() else { return };
//!     ui.add(egui::Slider::new(&mut rectangle.width, 50.0..=500.0).prefix("Width: "));
//!     ui.add(egui::Slider::new(&mut rectangle.height, 50.0..=500.0).prefix("Height: "));
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

mod editing;
mod editor;
mod editor_window;
mod entity_management;
mod entity_upgrading;
pub mod exclusive_systems;
pub mod knobs;
mod level_files_manager;
pub mod level_files_upgrading;
mod level_index;
mod populating;
mod specs_registration;
// #[cfg(feature = "vpeol")]
pub mod vpeol;
#[cfg(feature = "vpeol_2d")]
pub mod vpeol_2d;
#[cfg(feature = "vpeol_3d")]
pub mod vpeol_3d;

use std::any::{Any, TypeId};
use std::path::Path;
use std::sync::Arc;

use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy::utils::HashMap;

pub mod prelude {
    pub use crate::editing::{YoleckEdit, YoleckUi};
    pub use crate::editor::{YoleckEditorState, YoleckPassedData, YoleckSyncWithEditorState};
    pub use crate::entity_management::{YoleckLoadingCommand, YoleckRawLevel};
    pub use crate::entity_upgrading::YoleckEntityUpgradingPlugin;
    pub use crate::knobs::YoleckKnobs;
    pub use crate::level_index::{YoleckLevelIndex, YoleckLevelIndexEntry};
    pub use crate::populating::{YoleckMarking, YoleckPopulate};
    pub use crate::specs_registration::{YoleckComponent, YoleckEntityType};
    pub use crate::{
        YoleckBelongsToLevel, YoleckExtForApp, YoleckPluginForEditor, YoleckPluginForGame,
    };
    pub use bevy_yoleck_macros::YoleckComponent;
}

pub use self::editing::YoleckEditMarker;
pub use self::editor::YoleckDirective;
pub use self::editor::YoleckEditorEvent;
use self::editor::YoleckEditorState;
pub use self::editor_window::YoleckEditorSection;

use self::entity_management::{EntitiesToPopulate, YoleckLoadingCommand, YoleckRawLevel};
use self::entity_upgrading::YoleckEntityUpgrading;
use self::exclusive_systems::YoleckExclusiveSystemsPlugin;
use self::knobs::YoleckKnobsCache;
pub use self::level_files_manager::YoleckEditorLevelsDirectoryPath;
use self::level_index::YoleckLevelIndex;
pub use self::populating::{YoleckPopulateContext, YoleckSystemMarker};
use self::specs_registration::{YoleckComponentHandler, YoleckEntityType};
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
        app.init_resource::<YoleckEntityConstructionSpecs>();
        app.insert_resource(YoleckLoadingCommand::NoCommand);
        app.add_asset::<YoleckRawLevel>();
        app.add_asset_loader(entity_management::YoleckLevelAssetLoader);
        app.add_asset::<YoleckLevelIndex>();
        app.add_asset_loader(level_index::YoleckLevelIndexLoader);

        app.configure_sets(
            Update,
            (
                YoleckSystemSet::ProcessRawEntities,
                YoleckSystemSet::RunPopulateSchedule,
            )
                .chain(),
        );

        app.add_systems(
            Update,
            (
                entity_management::yoleck_process_raw_entries,
                apply_deferred,
            )
                .chain()
                .in_set(YoleckSystemSet::ProcessRawEntities),
        );
        app.insert_resource(EntitiesToPopulate(Default::default()));
        app.add_systems(
            Update,
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
        app.add_systems(Update, entity_management::yoleck_process_loading_command);
        app.add_schedule(YoleckSchedule::Populate, Schedule::new());
        app.add_schedule(YoleckSchedule::OverrideCommonComponents, Schedule::new());
    }
}

impl Plugin for YoleckPluginForGame {
    fn build(&self, app: &mut App) {
        app.add_state::<YoleckEditorState>();
        app.add_systems(
            Startup,
            |mut state: ResMut<NextState<YoleckEditorState>>| {
                state.set(YoleckEditorState::GameActive);
            },
        );
        app.add_plugins(YoleckPluginBase);
    }
}

impl Plugin for YoleckPluginForEditor {
    fn build(&self, app: &mut App) {
        app.add_state::<YoleckEditorState>();
        app.add_event::<YoleckEditorEvent>();
        app.add_plugins(YoleckPluginBase);
        app.add_plugins(YoleckExclusiveSystemsPlugin);
        app.init_resource::<YoleckEditSystems>();
        app.insert_resource(YoleckKnobsCache::default());
        app.insert_resource(YoleckState {
            level_needs_saving: false,
        });
        app.insert_resource(YoleckEditorLevelsDirectoryPath(
            Path::new(".").join("assets").join("levels"),
        ));
        app.insert_resource(YoleckEditorSections::default());
        app.add_event::<YoleckDirective>();
        app.add_systems(
            Update,
            editor_window::yoleck_editor_window.after(YoleckSystemSet::ProcessRawEntities),
        );

        app.add_schedule(
            YoleckInternalSchedule::UpdateManagedDataFromComponents,
            Schedule::new(),
        );
    }
}

pub trait YoleckExtForApp {
    /// Add a type of entity that can be edited in Yoleck's level editor.
    ///
    /// ```no_run
    /// # use bevy::prelude::*;
    /// # use bevy_yoleck::prelude::*;
    /// # use serde::{Deserialize, Serialize};
    /// # #[derive(Default, Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
    /// # struct Component1;
    /// # type Component2 = Component1;
    /// # type Component3 = Component1;
    /// # let mut app = App::new();
    /// app.add_yoleck_entity_type({
    ///     YoleckEntityType::new("MyEntityType")
    ///         .with::<Component1>()
    ///         .with::<Component2>()
    ///         .with::<Component3>()
    /// });
    /// ```
    fn add_yoleck_entity_type(&mut self, entity_type: YoleckEntityType);

    /// Add a system for editing Yoleck components in the level editor.
    ///
    /// ```no_run
    /// # use bevy::prelude::*;
    /// # use bevy_yoleck::prelude::*;
    /// # use serde::{Deserialize, Serialize};
    /// # #[derive(Default, Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
    /// # struct Component1;
    /// # let mut app = App::new();
    ///
    /// app.add_yoleck_edit_system(edit_component1);
    ///
    /// fn edit_component1(mut ui: NonSendMut<YoleckUi>, mut edit: YoleckEdit<&mut Component1>) {
    ///     let Ok(component1) = edit.get_single_mut() else { return };
    ///     // Edit `component1` with the `ui`
    /// }
    /// ```
    ///
    /// See [`YoleckEdit`](crate::editing::YoleckEdit).
    fn add_yoleck_edit_system<P>(&mut self, system: impl IntoSystem<(), (), P>);

    /// Get a Bevy schedule to add Yoleck populate systems on.
    ///
    /// ```no_run
    /// # use bevy::prelude::*;
    /// # use bevy_yoleck::prelude::*;
    /// # use serde::{Deserialize, Serialize};
    /// # #[derive(Default, Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
    /// # struct Component1;
    /// # let mut app = App::new();
    ///
    /// app.yoleck_populate_schedule_mut().add_systems(populate_component1);
    ///
    /// fn populate_component1(mut populate: YoleckPopulate<&Component1>) {
    ///     populate.populate(|_ctx, mut cmd, component1| {
    ///         // Add Bevy components derived from `component1` to `cmd`.
    ///     });
    /// }
    /// ```
    fn yoleck_populate_schedule_mut(&mut self) -> &mut Schedule;

    /// Register a function that upgrades entities from a previous version of the app format.
    ///
    /// This should only be called _after_ adding
    /// [`YoleckEntityUpgradingPlugin`](crate::entity_upgrading::YoleckEntityUpgradingPlugin). See
    /// that plugin's docs for more info.
    fn add_yoleck_entity_upgrade(
        &mut self,
        to_version: usize,
        upgrade_dlg: impl 'static + Send + Sync + Fn(&str, &mut serde_json::Value),
    );

    /// Register a function that upgrades entities of a specific type from a previous version of
    /// the app format.
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

    fn add_yoleck_edit_system<P>(&mut self, system: impl IntoSystem<(), (), P>) {
        let mut system = IntoSystem::into_system(system);
        system.initialize(&mut self.world);
        let mut edit_systems = self
            .world
            .get_resource_or_insert_with(YoleckEditSystems::default);
        edit_systems.edit_systems.push(Box::new(system));
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

    /// The type of the Yoleck entity, as registered with
    /// [`add_yoleck_entity_type`](YoleckExtForApp::add_yoleck_entity_type).
    ///
    /// This defines the Yoleck components that can be edited for the entity.
    pub type_name: String,

    lifecycle_status: YoleckEntityLifecycleStatus,

    components_data: HashMap<TypeId, BoxedAny>,
}

/// A marker for entities that belongs to the Yoleck level and should be despawned with it.
///
/// Yoleck already adds this automatically to entities created from the editor. The game itself
/// should this to entities created during gameplay, like bullets or spawned enemeis, so that
/// they'll be despawned when a playtest is finished or restarted.
///
/// When despawning a level as part of the game's flow (e.g. - before loading the next level), use
/// this marker to decide which entities to `despawn_recursive`.
///
/// There is no need to add this to child entities of entities that already has this marker,
/// because Yoleck will use `despawn_recursive` internally and so should actual games when
/// despawning these entities.
#[derive(Component)]
pub struct YoleckBelongsToLevel;

pub enum YoleckEntityLifecycleStatus {
    Synchronized,
    JustCreated,
    JustChanged,
}

#[derive(Default, Resource)]
struct YoleckEditSystems {
    edit_systems: Vec<Box<dyn System<In = (), Out = ()>>>,
}

impl YoleckEditSystems {
    pub(crate) fn run_systems(&mut self, world: &mut World) {
        for system in self.edit_systems.iter_mut() {
            system.run((), world);
            system.apply_deferred(world);
        }
    }
}

pub(crate) struct YoleckEntityTypeInfo {
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
pub(crate) struct YoleckState {
    level_needs_saving: bool,
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
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum YoleckInternalSchedule {
    UpdateManagedDataFromComponents,
}

/// Schedules for [Yoleck's populate schedule](YoleckExtForApp::yoleck_populate_schedule_mut).
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub enum YoleckSchedule {
    /// This is where most user defined populate systems should reside.
    Populate,
    /// Since many bundles add their own transform and visibility components, systems that override
    /// them explicitly need to go here.
    OverrideCommonComponents,
}
