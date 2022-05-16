use std::marker::PhantomData;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy_egui::egui;

use crate::{BoxedAny, YoleckEditContext, YoleckPopulateContext, YoleckSource};

pub enum YoleckOnEditorResult {
    Unchanged,
    Changed,
}

pub trait YoleckTypeHandlerTrait: Send + Sync {
    fn type_name(&self) -> &str;
    fn make_concrete(&self, data: serde_json::Value) -> serde_json::Result<BoxedAny>;
    fn populate(&self, data: &BoxedAny, ctx: &YoleckPopulateContext, cmd: &mut EntityCommands);
    #[allow(clippy::too_many_arguments)]
    fn on_editor(
        &self,
        data: &mut BoxedAny,
        comparison_cache: &mut Option<(Entity, BoxedAny)>,
        entity: Entity,
        editor_ctx: &YoleckEditContext,
        ui: &mut egui::Ui,
        populate_ctx: &YoleckPopulateContext,
        commands: &mut Commands,
    ) -> YoleckOnEditorResult;
    fn make_raw(&self, data: &BoxedAny) -> serde_json::Value;
}

pub(crate) struct YoleckTypeHandlerFor<T: YoleckSource> {
    pub(crate) _phantom_data: PhantomData<fn() -> T>,
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

    fn populate(&self, data: &BoxedAny, ctx: &YoleckPopulateContext, cmd: &mut EntityCommands) {
        let concrete = data.downcast_ref::<T>().unwrap();
        concrete.populate(ctx, cmd);
    }

    fn on_editor(
        &self,
        data: &mut BoxedAny,
        comparison_cache: &mut Option<(Entity, BoxedAny)>,
        entity: Entity,
        editor_ctx: &YoleckEditContext,
        ui: &mut egui::Ui,
        populate_ctx: &YoleckPopulateContext,
        commands: &mut Commands,
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
        if populate_ctx.is_first_time() || check_against != concrete {
            *check_against = concrete.clone();
            let mut cmd = commands.entity(entity);
            concrete.populate(populate_ctx, &mut cmd);
            YoleckOnEditorResult::Changed
        } else {
            YoleckOnEditorResult::Unchanged
        }
    }

    fn make_raw(&self, data: &BoxedAny) -> serde_json::Value {
        let concrete = data.downcast_ref::<T>().unwrap();
        serde_json::to_value(concrete).unwrap()
    }
}
