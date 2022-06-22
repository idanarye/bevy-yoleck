use std::marker::PhantomData;

use bevy::prelude::*;
use bevy_egui::egui;
use serde::{Deserialize, Serialize};

use crate::api::YoleckUiForEditSystem;
use crate::{BoxedAny, YoleckManaged};

pub enum YoleckEditingResult {
    Unchanged,
    Changed,
}

#[doc(hidden)]
pub trait YoleckTypeHandlerTrait: Send + Sync {
    fn type_name(&self) -> &str;
    fn make_concrete(&self, data: serde_json::Value) -> serde_json::Result<BoxedAny>;
    fn make_raw(&self, data: &BoxedAny) -> serde_json::Value;
    fn initialize_systems(&mut self, world: &mut World);
    fn run_edit_systems(
        &mut self,
        world: &mut World,
        ui: &mut egui::Ui,
        edited_entity: Entity,
        comparison_cache: &mut Option<(Entity, BoxedAny)>,
    ) -> YoleckEditingResult;
    fn run_populate_systems(&mut self, world: &mut World);
}

/// Descriptor for how Yoleck handles an entity type.
pub struct YoleckTypeHandler<T> {
    name: String,
    edit_systems: Vec<Box<dyn System<In = (), Out = ()>>>,
    populate_systems: Vec<Box<dyn System<In = (), Out = ()>>>,
    pub(crate) _phantom_data: PhantomData<fn() -> T>,
}

impl<T> YoleckTypeHandler<T> {
    /// Create a new type handler.
    ///
    /// The name will be used the level files, so changing it may break existing levels.
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            edit_systems: Default::default(),
            populate_systems: Default::default(),
            _phantom_data: Default::default(),
        }
    }

    /// Call a function to modify the handler.
    ///
    /// These functions usually come from modules like [vpeol](crate::vpeol) or similar
    /// library code, that can implement common edit and/or populate systems (like an entity's
    /// position)
    pub fn with(self, adapter: impl FnOnce(Self) -> Self) -> Self {
        adapter(self)
    }

    /// Add an edit system to the handler.
    ///
    /// Edit systems are the UI the level editor uses to edit the entity.
    pub fn edit_with<P>(mut self, system: impl IntoSystem<(), (), P>) -> Self {
        self.edit_systems
            .push(Box::new(IntoSystem::into_system(system)));
        self
    }

    /// Add a populate system to the handler.
    ///
    /// Populate system are used to populate the entity with components, either in game or when the
    /// entity is modified in the editor.
    pub fn populate_with<P>(mut self, system: impl IntoSystem<(), (), P>) -> Self {
        self.populate_systems
            .push(Box::new(IntoSystem::into_system(system)));
        self
    }
}

impl<T> YoleckTypeHandlerTrait for YoleckTypeHandler<T>
where
    T: 'static,
    T: Send,
    T: Sync,
    T: Clone,
    T: PartialEq,
    T: Serialize,
    T: for<'a> Deserialize<'a>,
{
    fn type_name(&self) -> &str {
        &self.name
    }

    fn make_concrete(&self, data: serde_json::Value) -> serde_json::Result<BoxedAny> {
        let concrete: T = serde_json::from_value(data)?;
        let dynamic: BoxedAny = Box::new(concrete);
        dynamic.downcast_ref::<T>().unwrap();
        Ok(dynamic)
    }

    fn make_raw(&self, data: &BoxedAny) -> serde_json::Value {
        let concrete = data.downcast_ref::<T>().unwrap();
        serde_json::to_value(concrete).unwrap()
    }

    fn initialize_systems(&mut self, world: &mut World) {
        for system in self.edit_systems.iter_mut() {
            system.initialize(world);
        }
        for system in self.populate_systems.iter_mut() {
            system.initialize(world);
        }
    }

    fn run_edit_systems(
        &mut self,
        world: &mut World,
        ui: &mut egui::Ui,
        edited_entity: Entity,
        comparison_cache: &mut Option<(Entity, BoxedAny)>,
    ) -> YoleckEditingResult {
        let before_edit = match comparison_cache {
            Some((cached_entity, cached_data)) if *cached_entity == edited_entity => cached_data
                .downcast_mut::<T>()
                .expect("Yoleck source type was changed but cached entity was not cleared"),
            _ => {
                let concrete = world
                    .get::<YoleckManaged>(edited_entity)
                    .unwrap()
                    .data
                    .downcast_ref::<T>()
                    .unwrap();
                *comparison_cache = Some((edited_entity, Box::new(concrete.clone())));
                let (_, cached_data) = comparison_cache.as_mut().unwrap();
                cached_data
                    .downcast_mut::<T>()
                    .expect("Data was just set to that type")
            }
        };

        let frame = egui::Frame::none();
        let mut prepared = frame.begin(ui);
        let content_ui = std::mem::replace(
            &mut prepared.content_ui,
            ui.child_ui(egui::Rect::EVERYTHING, *ui.layout()),
        );
        world.insert_resource(YoleckUiForEditSystem(content_ui));
        for system in self.edit_systems.iter_mut() {
            system.run((), world);
            system.apply_buffers(world);
        }
        prepared.content_ui = world.remove_resource::<YoleckUiForEditSystem>().unwrap().0;
        prepared.end(ui);

        let after_edit = world
            .get::<YoleckManaged>(edited_entity)
            .unwrap()
            .data
            .downcast_ref::<T>()
            .unwrap();
        if before_edit != after_edit {
            *before_edit = after_edit.clone();
            YoleckEditingResult::Changed
        } else {
            YoleckEditingResult::Unchanged
        }
    }

    fn run_populate_systems(&mut self, world: &mut World) {
        for system in self.populate_systems.iter_mut() {
            system.run((), world);
            system.apply_buffers(world);
        }
    }
}
