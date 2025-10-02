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
//! * Register populate systems on [`YoleckSchedule::Populate`]
//! * If the application starts in editor mode:
//!   * Add the `EguiPlugin` plugin.
//!   * Add the [`YoleckPluginForEditor`] plugin.
//!   * Use [`YoleckSyncWithEditorState`](crate::editor::YoleckSyncWithEditorState) to synchronize
//!     the game's state with the [`YoleckEditorState`] (optional but highly recommended)
//! * If the application starts in game mode:
//!   * Add the [`YoleckPluginForGame`] plugin.
//!   * Use the [`YoleckLevelIndex`] asset to determine the list of available levels (optional)
//!   * Spawn an entity with the [`YoleckLoadLevel`](entity_management::YoleckLoadLevel) component
//!     to load the level. Note that the level can be unloaded by despawning that entity or by
//!     removing the [`YoleckKeepLevel`] component that will automatically be added to it.
//!
//! To support picking and moving entities in the viewport with the mouse, check out the
//! [`vpeol_2d`] and [`vpeol_3d`] modules. After adding the appropriate feature flag
//! (`vpeol_2d`/`vpeol_3d`), import their types from
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
//!         app.add_plugins(EguiPlugin {
//!             enable_multipass_for_primary_context: true,
//!         });
//!         app.add_plugins(YoleckSyncWithEditorState {
//!             when_editor: GameState::Editor,
//!             when_game: GameState::Game,
//!         });
//!         app.add_plugins(YoleckPluginForEditor);
//!     } else {
//!         app.add_plugins(YoleckPluginForGame);
//!         app.init_state::<GameState>();
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
//!     app.add_systems(YoleckSchedule::Populate, populate_rectangle);
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
//!     commands.spawn(Camera2d::default());
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
//!         cmd.insert(Sprite {
//!             color: bevy::color::palettes::css::RED.into(),
//!             custom_size: Some(Vec2::new(rectangle.width, rectangle.height)),
//!             ..Default::default()
//!         });
//!     });
//! }
//!
//! fn edit_rectangle(mut ui: ResMut<YoleckUi>, mut edit: YoleckEdit<&mut Rectangle>) {
//!     let Ok(mut rectangle) = edit.single_mut() else { return };
//!     ui.add(egui::Slider::new(&mut rectangle.width, 50.0..=500.0).prefix("Width: "));
//!     ui.add(egui::Slider::new(&mut rectangle.height, 50.0..=500.0).prefix("Height: "));
//! }
//!
//! fn load_first_level(
//!     mut level_index_handle: Local<Option<Handle<YoleckLevelIndex>>>,
//!     asset_server: Res<AssetServer>,
//!     level_index_assets: Res<Assets<YoleckLevelIndex>>,
//!     mut commands: Commands,
//!     mut game_state: ResMut<NextState<GameState>>,
//! ) {
//!     // Keep the handle in local resource, so that Bevy will not unload the level index asset
//!     // between frames.
//!     let level_index_handle = level_index_handle
//!         .get_or_insert_with(|| asset_server.load("levels/index.yoli"))
//!         .clone();
//!     let Some(level_index) = level_index_assets.get(&level_index_handle) else {
//!         // During the first invocation of this system, the level index asset is not going to be
//!         // loaded just yet. Since this system is going to run on every frame during the Loading
//!         // state, it just has to keep trying until it starts in a frame where it is loaded.
//!         return;
//!     };
//!     // A proper game would have a proper level progression system, but here we are just
//!     // taking the first level and loading it.
//!     let level_handle: Handle<YoleckRawLevel> =
//!         asset_server.load(&format!("levels/{}", level_index[0].filename));
//!     commands.spawn(YoleckLoadLevel(level_handle));
//!     game_state.set(GameState::Game);
//! }
//! ```

mod editing;
mod editor;
mod editor_window;
mod entity_management;
mod entity_upgrading;
mod entity_uuid;
mod errors;
pub mod exclusive_systems;
pub mod knobs;
mod level_files_manager;
pub mod level_files_upgrading;
mod level_index;
mod picking_helpers;
mod populating;
mod specs_registration;
mod util;
#[cfg(feature = "vpeol")]
pub mod vpeol;
#[cfg(feature = "vpeol_2d")]
pub mod vpeol_2d;
#[cfg(feature = "vpeol_3d")]
pub mod vpeol_3d;

use std::any::{Any, TypeId};
use std::path::Path;
use std::sync::Arc;

use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::{EntityCommands, SystemId};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

pub mod prelude {
    pub use crate::editing::{YoleckEdit, YoleckUi};
    pub use crate::editor::{YoleckEditorState, YoleckPassedData, YoleckSyncWithEditorState};
    pub use crate::entity_management::{YoleckKeepLevel, YoleckLoadLevel, YoleckRawLevel};
    pub use crate::entity_upgrading::YoleckEntityUpgradingPlugin;
    pub use crate::entity_uuid::{YoleckEntityUuid, YoleckUuidRegistry};
    pub use crate::knobs::YoleckKnobs;
    pub use crate::level_index::{YoleckLevelIndex, YoleckLevelIndexEntry};
    pub use crate::populating::{YoleckMarking, YoleckPopulate};
    pub use crate::specs_registration::{YoleckComponent, YoleckEntityType};
    pub use crate::{
        YoleckBelongsToLevel, YoleckExtForApp, YoleckLevelInEditor, YoleckLevelInPlaytest,
        YoleckLevelJustLoaded, YoleckPluginForEditor, YoleckPluginForGame, YoleckSchedule,
    };
    pub use bevy_yoleck_macros::YoleckComponent;
}

pub use self::editing::YoleckEditMarker;
pub use self::editor::YoleckDirective;
pub use self::editor::YoleckEditorEvent;
use self::editor::YoleckEditorState;
pub use self::editor_window::YoleckEditorSection;
pub use self::picking_helpers::*;

use self::entity_management::{EntitiesToPopulate, YoleckRawLevel};
use self::entity_upgrading::YoleckEntityUpgrading;
use self::exclusive_systems::YoleckExclusiveSystemsPlugin;
use self::knobs::YoleckKnobsCache;
pub use self::level_files_manager::YoleckEditorLevelsDirectoryPath;
pub use self::level_index::YoleckEditableLevels;
use self::level_index::YoleckLevelIndex;
pub use self::populating::{YoleckPopulateContext, YoleckSystemMarker};
use self::prelude::{YoleckKeepLevel, YoleckUuidRegistry};
use self::specs_registration::{YoleckComponentHandler, YoleckEntityType};
use self::util::EditSpecificResources;
pub use bevy_egui;
pub use bevy_egui::egui;

struct YoleckPluginBase;
pub struct YoleckPluginForGame;
pub struct YoleckPluginForEditor;

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
enum YoleckSystems {
    ProcessRawEntities,
    RunPopulateSchedule,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub(crate) struct YoleckRunEditSystems;

impl Plugin for YoleckPluginBase {
    fn build(&self, app: &mut App) {
        app.init_resource::<YoleckEntityConstructionSpecs>();
        app.insert_resource(YoleckUuidRegistry(Default::default()));
        app.register_asset_loader(entity_management::YoleckLevelAssetLoader);
        app.init_asset::<YoleckRawLevel>();
        app.register_asset_loader(level_index::YoleckLevelIndexLoader);
        app.init_asset::<YoleckLevelIndex>();

        app.configure_sets(
            Update,
            (
                YoleckSystems::ProcessRawEntities,
                YoleckSystems::RunPopulateSchedule,
            )
                .chain(),
        );

        app.add_systems(
            Update,
            (
                entity_management::yoleck_process_raw_entries,
                ApplyDeferred,
                (
                    entity_management::yoleck_run_level_loaded_schedule,
                    entity_management::yoleck_remove_just_loaded_marker_from_levels,
                    ApplyDeferred,
                )
                    .chain()
                    .run_if(
                        |freshly_loaded_level_entities: Query<
                            (),
                            (With<YoleckLevelJustLoaded>, Without<YoleckLevelInEditor>),
                        >| { !freshly_loaded_level_entities.is_empty() },
                    ),
            )
                .chain()
                .in_set(YoleckSystems::ProcessRawEntities),
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
                .in_set(YoleckSystems::RunPopulateSchedule),
        );
        app.add_systems(
            Update,
            ((
                entity_management::process_unloading_command,
                entity_management::process_loading_command,
                ApplyDeferred,
            )
                .chain()
                .before(YoleckSystems::ProcessRawEntities),),
        );
        app.add_schedule(Schedule::new(YoleckSchedule::Populate));
        app.add_schedule(Schedule::new(YoleckSchedule::LevelLoaded));
        app.add_schedule(Schedule::new(YoleckSchedule::OverrideCommonComponents));
    }
}

impl Plugin for YoleckPluginForGame {
    fn build(&self, app: &mut App) {
        app.init_state::<YoleckEditorState>();
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
        app.init_state::<YoleckEditorState>();
        app.add_message::<YoleckEditorEvent>();
        app.add_plugins(YoleckPluginBase);
        app.add_plugins(YoleckExclusiveSystemsPlugin);
        app.init_resource::<YoleckEditSystems>();
        app.insert_resource(YoleckKnobsCache::default());
        let level_being_edited = app
            .world_mut()
            .spawn((YoleckLevelInEditor, YoleckKeepLevel))
            .id();
        app.insert_resource(YoleckState {
            level_being_edited,
            level_needs_saving: false,
        });
        app.insert_resource(YoleckEditorLevelsDirectoryPath(
            Path::new(".").join("assets").join("levels"),
        ));
        app.insert_resource(YoleckEditorSections::default());
        app.insert_resource(EditSpecificResources::new().with(YoleckEditableLevels {
            levels: Default::default(),
        }));
        app.add_message::<YoleckDirective>();
        app.configure_sets(
            Update,
            YoleckRunEditSystems.after(YoleckSystems::ProcessRawEntities),
        );
        app.add_systems(
            EguiPrimaryContextPass,
            editor_window::yoleck_editor_window.in_set(YoleckRunEditSystems),
        );

        app.add_schedule(Schedule::new(
            YoleckInternalSchedule::UpdateManagedDataFromComponents,
        ));
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
    /// fn edit_component1(mut ui: ResMut<YoleckUi>, mut edit: YoleckEdit<&mut Component1>) {
    ///     let Ok(component1) = edit.single_mut() else { return };
    ///     // Edit `component1` with the `ui`
    /// }
    /// ```
    ///
    /// See [`YoleckEdit`](crate::editing::YoleckEdit).
    fn add_yoleck_edit_system<P>(&mut self, system: impl 'static + IntoSystem<(), (), P>);

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
            .world_mut()
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
            has_uuid: entity_type.has_uuid,
        };

        let mut construction_specs = self
            .world_mut()
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

    fn add_yoleck_edit_system<P>(&mut self, system: impl 'static + IntoSystem<(), (), P>) {
        let system_id = self.world_mut().register_system(system);
        let mut edit_systems = self
            .world_mut()
            .get_resource_or_insert_with(YoleckEditSystems::default);
        edit_systems.edit_systems.push(system_id);
    }

    fn add_yoleck_entity_upgrade(
        &mut self,
        to_version: usize,
        upgrade_dlg: impl 'static + Send + Sync + Fn(&str, &mut serde_json::Value),
    ) {
        let mut entity_upgrading = self.world_mut().get_resource_mut::<YoleckEntityUpgrading>()
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
/// should add this to entities created during gameplay, like bullets or spawned enemeis, so that
/// they'll be despawned when a playtest is finished or restarted.
///
/// When removing a [`YoleckKeepLevel`] from entity (or removing the entire entity), Yoleck will
/// automatically despawn all the entities that have this component and point to that level.
///
/// There is no need to add this to child entities of entities that already has this marker,
/// because Bevy will already despawn them when despawning their parent.
#[derive(Component, Debug, Clone)]
pub struct YoleckBelongsToLevel {
    /// The entity which was used with [`YoleckLoadLevel`](entity_management::YoleckLoadLevel) to
    /// load the level that this entity belongs to.
    pub level: Entity,
}

pub enum YoleckEntityLifecycleStatus {
    Synchronized,
    JustCreated,
    JustChanged,
}

#[derive(Default, Resource)]
struct YoleckEditSystems {
    edit_systems: Vec<SystemId>,
}

impl YoleckEditSystems {
    pub(crate) fn run_systems(&mut self, world: &mut World) {
        for system_id in self.edit_systems.iter() {
            world
                .run_system(*system_id)
                .expect("edit systems handled by Yoleck - system should been properly handled");
        }
    }
}

pub(crate) struct YoleckEntityTypeInfo {
    pub name: String,
    pub components: Vec<TypeId>,
    #[allow(clippy::type_complexity)]
    pub(crate) on_init:
        Vec<Box<dyn 'static + Sync + Send + Fn(YoleckEditorState, &mut EntityCommands)>>,
    pub has_uuid: bool,
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
    level_being_edited: Entity,
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
/// app.world_mut().resource_mut::<YoleckEditorSections>().0.push((|world: &mut World| {
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

/// Schedules for user code to do the actual entity/level population after Yoleck spawns the level
/// "skeleton".
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub enum YoleckSchedule {
    /// This is where user defined populate systems should reside.
    ///
    /// Note that populate systems, rather than directly trying to query the entities to be
    /// populated, should use [`YoleckPopulate`](crate::prelude::YoleckPopulate):
    ///
    /// ```no_run
    /// # use bevy::prelude::*;
    /// # use bevy_yoleck::prelude::*;
    /// # use serde::{Deserialize, Serialize};
    /// # #[derive(Default, Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
    /// # struct Component1;
    /// # let mut app = App::new();
    ///
    /// app.add_systems(YoleckSchedule::Populate, populate_component1);
    ///
    /// fn populate_component1(mut populate: YoleckPopulate<&Component1>) {
    ///     populate.populate(|_ctx, mut cmd, component1| {
    ///         // Add Bevy components derived from `component1` to `cmd`.
    ///     });
    /// }
    /// ```
    Populate,
    /// Right after all the level entities are loaded, but before any populate systems manage to
    /// run.
    LevelLoaded,
    /// Since many bundles add their own transform and visibility components, systems that override
    /// them explicitly need to go here.
    OverrideCommonComponents,
}

/// Automatically added to level entities that are being edited in the level editor.
#[derive(Component)]
pub struct YoleckLevelInEditor;

/// Automatically added to level entities that are being play-tested in the level editor.
///
/// Note that this only gets added to the levels that are launched from the editor UI. If game
/// systems load new levels during the play-test, this component will not be added to them.
#[derive(Component)]
pub struct YoleckLevelInPlaytest;

/// During the [`YoleckSchedule::LevelLoaded`] schedule, this component marks the level entities
/// that were just loaded and triggered that schedule.
///
/// Note that this component will be removed after that schedule finishes running - it should not
/// be relied on in systems outside that schedule.
#[derive(Component)]
pub struct YoleckLevelJustLoaded;
