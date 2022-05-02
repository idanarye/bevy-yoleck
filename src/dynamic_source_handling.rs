use std::marker::PhantomData;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy_egui::egui;
use serde::{Deserialize, Serialize};

use crate::{BoxedAny, YoleckEditContext, YoleckPopulateContext, YoleckSource};

pub trait YoleckTypeHandlerTrait: Send + Sync {
    fn type_name(&self) -> &str;
    fn make_concrete(&self, data: serde_json::Value) -> serde_json::Result<BoxedAny>;
    fn populate(&self, data: &BoxedAny, ctx: &YoleckPopulateContext, cmd: &mut EntityCommands);
    fn on_editor(
        &self,
        data: &mut BoxedAny,
        entity: Entity,
        editor_ctx: &YoleckEditContext,
        ui: &mut egui::Ui,
        populate_ctx: &YoleckPopulateContext,
        commands: &mut Commands,
    );
    fn make_raw(&self, data: &BoxedAny) -> serde_json::Value;
}

pub(crate) struct YoleckTypeHandlerFor<T>
where
    T: 'static,
    T: YoleckSource,
    T: Serialize,
    for<'de> T: Deserialize<'de>,
{
    pub(crate) type_name: String,
    pub(crate) _phantom_data: PhantomData<fn() -> T>,
}

impl<T> YoleckTypeHandlerTrait for YoleckTypeHandlerFor<T>
where
    T: 'static,
    T: YoleckSource,
    T: Serialize,
    for<'de> T: Deserialize<'de>,
{
    fn type_name(&self) -> &str {
        &self.type_name
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
        entity: Entity,
        editor_ctx: &YoleckEditContext,
        ui: &mut egui::Ui,
        populate_ctx: &YoleckPopulateContext,
        commands: &mut Commands,
    ) {
        let concrete = data.downcast_mut::<T>().unwrap();
        concrete.edit(editor_ctx, ui);
        concrete.populate(populate_ctx, &mut commands.entity(entity));
    }

    fn make_raw(&self, data: &BoxedAny) -> serde_json::Value {
        let concrete = data.downcast_ref::<T>().unwrap();
        serde_json::to_value(concrete).unwrap()
    }
}
