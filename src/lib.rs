mod api;
mod dynamic_source_handling;
mod editor;
mod editor_window;
mod entity_management;
mod level_files_manager;
mod level_index;
#[cfg(feature = "tools_2d")]
pub mod tools_2d;

use std::any::Any;
use std::path::Path;

use bevy::prelude::*;
use bevy::utils::HashMap;

use self::api::PopulateReason;
pub use self::api::{YoleckEditContext, YoleckEditorState, YoleckPopulateContext, YoleckSource};
use self::dynamic_source_handling::{YoleckTypeHandlerFor, YoleckTypeHandlerTrait};
pub use self::editor::YoleckDirective;
pub use self::editor_window::YoleckEditorSection;
pub use self::entity_management::{
    YoleckEntryHeader, YoleckLoadingCommand, YoleckRawEntry, YoleckRawLevel,
};
pub use self::level_files_manager::YoleckEditorLevelsDirectoryPath;
pub use self::level_index::{YoleckLevelIndex, YoleckLevelIndexEntry};
pub use bevy_egui;
pub use bevy_egui::egui;

struct YoleckPluginBase;
pub struct YoleckPluginForGame;
pub struct YoleckPluginForEditor;

impl Plugin for YoleckPluginBase {
    fn build(&self, app: &mut App) {
        app.insert_resource(YoleckLoadingCommand::NoCommand);
        app.init_resource::<YoleckTypeHandlers>();
        app.add_asset::<YoleckRawLevel>();
        app.add_asset_loader(entity_management::YoleckLevelAssetLoader);
        app.add_asset::<YoleckLevelIndex>();
        app.add_asset_loader(level_index::YoleckLevelIndexLoader);
        app.add_system(entity_management::yoleck_process_raw_entries);
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
        app.add_plugin(YoleckPluginBase);
        app.insert_resource(YoleckState {
            entity_being_edited: None,
            level_needs_saving: false,
        });
        app.insert_resource(YoleckEditorLevelsDirectoryPath(
            Path::new(".").join("assets").join("levels"),
        ));
        app.insert_resource(YoleckEditorSections::default());
        app.add_event::<YoleckDirective>();
        app.add_system(editor_window::yoleck_editor_window.exclusive_system());
    }
}

pub trait YoleckExtForApp {
    fn add_yoleck_handler<T: YoleckSource>(&mut self);
}

impl YoleckExtForApp for App {
    fn add_yoleck_handler<T: YoleckSource>(&mut self) {
        let mut handlers = self
            .world
            .get_resource_or_insert_with(YoleckTypeHandlers::default);
        handlers.add_handler(T::handler());
    }
}

type BoxedAny = Box<dyn Send + Sync + Any>;

#[derive(Component)]
pub struct YoleckManaged {
    pub name: String,
    pub type_name: String,
    pub data: BoxedAny,
}

#[derive(Default)]
pub struct YoleckTypeHandlers {
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

pub struct YoleckState {
    entity_being_edited: Option<Entity>,
    level_needs_saving: bool,
}

impl YoleckState {
    pub fn entity_being_edited(&self) -> Option<Entity> {
        self.entity_being_edited
    }
}

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
