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
//!         app.add_state(GameState::Loading);
//!         // In editor mode Yoleck takes care of level loading. In game mode the game needs to
//!         // tell yoleck which levels to load and when.
//!         app.add_system_set(SystemSet::on_update(GameState::Loading).with_system(load_first_level));
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
//! #[derive(Debug, Clone, PartialEq, Eq, Hash)]
//! enum GameState {
//!     Loading,
//!     Game,
//!     Editor,
//! }
//!
//! fn setup_camera(mut commands: Commands) {
//!     commands.spawn_bundle(Camera2dBundle::default());
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
//!         cmd.insert_bundle(SpriteBundle {
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
//!     mut game_state: ResMut<State<GameState>>,
//! ) {
//!     let level_index_handle: Handle<YoleckLevelIndex> = asset_server.load("levels/index.yoli");
//!     if let Some(level_index) = level_index_assets.get(&level_index_handle) {
//!         // A proper game would have a proper level progression system, but here we are just
//!         // taking the first level and loading it.
//!         let level_handle: Handle<YoleckRawLevel> =
//!             asset_server.load(&format!("levels/{}", level_index[0].filename));
//!         *loading_command = YoleckLoadingCommand::FromAsset(level_handle);
//!         game_state.set(GameState::Game).unwrap();
//!     }
//! }
//! ```

mod api;
mod dynamic_source_handling;
mod editor;
mod editor_window;
mod entity_management;
mod knobs;
mod level_files_manager;
mod level_index;
#[cfg(feature = "vpeol")]
pub mod vpeol;
#[cfg(feature = "vpeol_2d")]
pub mod vpeol_2d;

use std::any::Any;
use std::path::Path;
use std::sync::Arc;

use bevy::prelude::*;
use bevy::utils::HashMap;

use self::api::YoleckUserSystemContext;
pub use self::api::{
    YoleckEdit, YoleckEditContext, YoleckEditorEvent, YoleckEditorState, YoleckKnobHandle,
    YoleckPopulate, YoleckPopulateContext, YoleckSyncWithEditorState,
};
pub use self::dynamic_source_handling::YoleckTypeHandler;
use self::dynamic_source_handling::YoleckTypeHandlerTrait;
pub use self::editor::YoleckDirective;
pub use self::editor_window::YoleckEditorSection;
pub use self::entity_management::{
    YoleckEntryHeader, YoleckLoadingCommand, YoleckRawEntry, YoleckRawLevel,
};
pub use self::knobs::{YoleckKnob, YoleckKnobsCache};
pub use self::level_files_manager::YoleckEditorLevelsDirectoryPath;
pub use self::level_index::{YoleckLevelIndex, YoleckLevelIndexEntry};
pub use bevy_egui;
pub use bevy_egui::egui;

struct YoleckPluginBase;
pub struct YoleckPluginForGame;
pub struct YoleckPluginForEditor;

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemLabel)]
enum YoleckSystemLabels {
    ProcessRawEntities,
}

impl Plugin for YoleckPluginBase {
    fn build(&self, app: &mut App) {
        app.insert_resource(YoleckUserSystemContext::Nope);
        app.insert_resource(YoleckLoadingCommand::NoCommand);
        app.init_resource::<YoleckTypeHandlers>();
        app.add_asset::<YoleckRawLevel>();
        app.add_asset_loader(entity_management::YoleckLevelAssetLoader);
        app.add_asset::<YoleckLevelIndex>();
        app.add_asset_loader(level_index::YoleckLevelIndexLoader);
        app.add_system(
            entity_management::yoleck_process_raw_entries
                .label(YoleckSystemLabels::ProcessRawEntities),
        );
        app.add_system(entity_management::yoleck_process_loading_command);
    }
}

impl Plugin for YoleckPluginForGame {
    fn build(&self, app: &mut App) {
        app.add_state(YoleckEditorState::GameActive);
        app.add_plugin(YoleckPluginBase);
    }
}

impl Plugin for YoleckPluginForEditor {
    fn build(&self, app: &mut App) {
        app.add_state(YoleckEditorState::EditorActive);
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
            editor_window::yoleck_editor_window
                .after(YoleckSystemLabels::ProcessRawEntities),
        );
    }
}

pub trait YoleckExtForApp {
    /// Register a [`YoleckTypeHandler`] to describe a type of entity that can be edited with Yoleck.
    fn add_yoleck_handler(&mut self, handler: impl 'static + YoleckTypeHandlerTrait);
}

impl YoleckExtForApp for App {
    fn add_yoleck_handler(&mut self, mut handler: impl 'static + YoleckTypeHandlerTrait) {
        handler.initialize_systems(&mut self.world);
        let mut handlers = self
            .world
            .get_resource_or_insert_with(YoleckTypeHandlers::default);
        handlers.add_handler(Box::new(handler));
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
    /// This is the entity's data. The [`YoleckTypeHandler`] is responsible for manipulating
    /// it, using the systems registered to it.
    pub data: BoxedAny,
}

#[derive(Default, Resource)]
struct YoleckTypeHandlers {
    type_handler_names: Vec<String>,
    type_handlers: HashMap<String, Box<dyn YoleckTypeHandlerTrait>>,
}

impl YoleckTypeHandlers {
    fn add_handler(&mut self, handler: Box<dyn YoleckTypeHandlerTrait>) {
        let type_name = handler.type_name().to_owned();
        match self.type_handlers.entry(type_name.clone()) {
            bevy::utils::hashbrown::hash_map::Entry::Occupied(_) => {
                panic!("Handler for {:?} already exists", type_name);
            }
            bevy::utils::hashbrown::hash_map::Entry::Vacant(entry) => {
                entry.insert(handler);
            }
        }
        self.type_handler_names.push(type_name);
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
