mod api;
mod dynamic_source_handling;
mod editor;
mod editor_window;
mod entity_management;
mod level_files_manager;
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
pub use self::entity_management::{YoleckEntryHeader, YoleckRawEntry};
pub use self::level_files_manager::YoleckEditorLevelsDirectoryPath;

pub struct YoleckPlugin;

impl Plugin for YoleckPlugin {
    fn build(&self, app: &mut App) {
        app.add_state(YoleckEditorState::EditorActive);
        app.insert_resource(YoleckState {
            entity_being_edited: None,
        });
        app.insert_resource(YoleckEditorLevelsDirectoryPath(
            Path::new(".").join("assets").join("levels"),
        ));
        app.insert_resource(YoleckEditorSections::default());
        app.add_event::<YoleckDirective>();
        app.add_system_set(
            SystemSet::on_update(YoleckEditorState::EditorActive)
                .with_system(editor_window::yoleck_editor_window.exclusive_system()),
        );
        app.add_system(entity_management::yoleck_process_raw_entries);
    }
}

type BoxedAny = Box<dyn Send + Sync + Any>;

#[derive(Component)]
pub struct YoleckManaged {
    pub name: String,
    pub type_name: String,
    pub data: BoxedAny,
}

pub struct YoleckTypeHandlers {
    type_handler_names: Vec<String>,
    type_handlers: HashMap<String, Box<dyn YoleckTypeHandlerTrait>>,
}

impl YoleckTypeHandlers {
    pub fn new(handlers: impl IntoIterator<Item = Box<dyn YoleckTypeHandlerTrait>>) -> Self {
        let mut result = Self {
            type_handler_names: Default::default(),
            type_handlers: Default::default(),
        };
        for handler in handlers {
            result.add_handler(handler);
        }
        result
    }

    pub fn add_handler(&mut self, handler: Box<dyn YoleckTypeHandlerTrait>) {
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
