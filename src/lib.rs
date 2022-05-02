mod api;
mod dynamic_source_handling;
mod editor;
mod entity_management;
#[cfg(feature = "tools_2d")]
pub mod tools_2d;

use std::any::Any;

use bevy::prelude::*;
use bevy::utils::HashMap;

use self::api::PopulateReason;
pub use self::api::{YoleckEditContext, YoleckEditorState, YoleckPopulateContext, YoleckSource};
use self::dynamic_source_handling::{YoleckTypeHandlerFor, YoleckTypeHandlerTrait};
pub use self::editor::YoleckDirective;
pub use self::entity_management::{YoleckEntryHeader, YoleckRawEntry};

pub struct YoleckPlugin;

impl Plugin for YoleckPlugin {
    fn build(&self, app: &mut App) {
        app.add_state(YoleckEditorState::EditorActive);
        app.insert_resource(YoleckState {
            entity_being_edited: None,
        });
        app.add_event::<YoleckDirective>();
        app.add_system_set(
            SystemSet::on_update(YoleckEditorState::EditorActive)
                .with_system(editor::yoleck_editor),
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
