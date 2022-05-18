use std::marker::PhantomData;

use bevy::prelude::*;
use bevy_egui::egui;

use crate::{BoxedAny, YoleckEditContext, YoleckSource};

pub enum YoleckOnEditorResult {
    Unchanged,
    Changed,
}

pub trait YoleckTypeHandlerTrait: Send + Sync {
    fn type_name(&self) -> &str;
    fn make_concrete(&self, data: serde_json::Value) -> serde_json::Result<BoxedAny>;
    #[allow(clippy::too_many_arguments)]
    fn on_editor(
        &self,
        data: &mut BoxedAny,
        comparison_cache: &mut Option<(Entity, BoxedAny)>,
        entity: Entity,
        editor_ctx: &YoleckEditContext,
        ui: &mut egui::Ui,
    ) -> YoleckOnEditorResult;
    fn make_raw(&self, data: &BoxedAny) -> serde_json::Value;
    fn initialize_systems(&mut self, world: &mut World);
    fn run_populate_systems(&mut self, world: &mut World);
}

pub struct YoleckTypeHandlerFor<T: YoleckSource> {
    pub(crate) _phantom_data: PhantomData<fn() -> T>,
    pub populate_systems: Vec<Box<dyn System<In = (), Out = ()>>>,
}

impl<T: YoleckSource> YoleckTypeHandlerFor<T> {
    pub fn populate_with<P>(mut self, system: impl IntoSystem<(), (), P>) -> Self {
        self.populate_systems
            .push(Box::new(IntoSystem::into_system(system)));
        self
    }
}

impl<T: YoleckSource> YoleckTypeHandlerTrait for YoleckTypeHandlerFor<T> {
    fn type_name(&self) -> &str {
        T::NAME
    }

    fn make_concrete(&self, data: serde_json::Value) -> serde_json::Result<BoxedAny> {
        let concrete: T = serde_json::from_value(data)?;
        let dynamic: BoxedAny = Box::new(concrete);
        dynamic.downcast_ref::<T>().unwrap();
        Ok(dynamic)
    }

    fn on_editor(
        &self,
        data: &mut BoxedAny,
        comparison_cache: &mut Option<(Entity, BoxedAny)>,
        entity: Entity,
        editor_ctx: &YoleckEditContext,
        ui: &mut egui::Ui,
    ) -> YoleckOnEditorResult {
        let concrete = data.downcast_mut::<T>().unwrap();
        let check_against = match comparison_cache {
            Some((cached_entity, cached_data)) if *cached_entity == entity => cached_data
                .downcast_mut::<T>()
                .expect("Yoleck source type was changed but cached entity was not cleared"),
            _ => {
                *comparison_cache = Some((entity, Box::new(concrete.clone())));
                let (_, cached_data) = comparison_cache.as_mut().unwrap();
                cached_data
                    .downcast_mut::<T>()
                    .expect("Data was just set to that type")
            }
        };
        concrete.edit(editor_ctx, ui);
        if check_against != concrete {
            *check_against = concrete.clone();
            YoleckOnEditorResult::Changed
        } else {
            YoleckOnEditorResult::Unchanged
        }
    }

    fn make_raw(&self, data: &BoxedAny) -> serde_json::Value {
        let concrete = data.downcast_ref::<T>().unwrap();
        serde_json::to_value(concrete).unwrap()
    }

    fn initialize_systems(&mut self, world: &mut World) {
        for system in self.populate_systems.iter_mut() {
            system.initialize(world);
        }
    }

    fn run_populate_systems(&mut self, world: &mut World) {
        for system in self.populate_systems.iter_mut() {
            system.run((), world);
            system.apply_buffers(world);
        }
    }
}
