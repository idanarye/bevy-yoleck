use std::any::Any;
use std::marker::PhantomData;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_egui::{egui, EguiContext};
use serde::{Deserialize, Serialize};

pub struct YoleckPlugin;

impl Plugin for YoleckPlugin {
    fn build(&self, app: &mut App) {
        app.add_state(YoleckEditorState::EditorActive);
        app.add_system_set(
            SystemSet::on_update(YoleckEditorState::EditorActive).with_system(yoleck_editor),
        );
        app.insert_resource(YoleckState {
            entity_being_edited: None,
            type_handlers: Default::default(),
        });
        app.add_system(yoleck_process_raws);
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum YoleckEditorState {
    EditorActive,
}

pub trait YoleckSource: Send + Sync {
    fn populate(&self, cmd: &mut EntityCommands);
    fn edit(&mut self, ui: &mut egui::Ui);
}

type BoxedAny = Box<dyn Send + Sync + Any>;

#[derive(Component)]
pub struct YoleckManaged {
    pub name: String,
    pub type_name: String,
    pub data: BoxedAny,
}

pub struct YoleckState {
    entity_being_edited: Option<Entity>,
    type_handlers: HashMap<String, Box<dyn YoleckTypeHandlerTrait>>,
}

impl YoleckState {
    pub fn add_handler<T: 'static>(&mut self, name: String)
    where
        T: YoleckSource,
        T: Serialize,
        for<'de> T: Deserialize<'de>,
    {
        self.type_handlers.insert(
            name,
            Box::new(YoleckTypeHandlerFor::<T> {
                _phantom_data: Default::default(),
            }),
        );
    }
}

fn yoleck_editor(
    mut egui_context: ResMut<EguiContext>,
    mut yoleck: ResMut<YoleckState>,
    mut yoleck_managed_query: Query<(Entity, &mut YoleckManaged)>,
    mut commands: Commands,
) {
    egui::Window::new("Level Editor").show(egui_context.ctx_mut(), |ui| {
        let yoleck = yoleck.as_mut();

        if ui.button("Save Level").clicked() {
            for (entity, yoleck_managed) in yoleck_managed_query.iter() {
                let handler = yoleck.type_handlers.get(&yoleck_managed.type_name).unwrap();
                println!(
                    "{:?}: {}",
                    entity,
                    serde_json::to_string(&handler.make_raw(&yoleck_managed.data)).unwrap()
                );
            }
        }

        for (entity, yoleck_managed) in yoleck_managed_query.iter() {
            ui.selectable_value(
                &mut yoleck.entity_being_edited,
                Some(entity),
                format!("{} {:?}", yoleck_managed.name, entity),
            );
        }
        if let Some(entity) = yoleck.entity_being_edited {
            if let Ok((_, mut yoleck_managed)) = yoleck_managed_query.get_mut(entity) {
                ui.text_edit_singleline(&mut yoleck_managed.name);
                let handler = yoleck.type_handlers.get(&yoleck_managed.type_name).unwrap();
                handler.on_editor(&mut yoleck_managed.data, entity, ui, &mut commands);
            }
        }
    });
}

pub trait YoleckTypeHandlerTrait: Send + Sync {
    fn make_concrete(&self, data: serde_json::Value) -> serde_json::Result<BoxedAny>;
    fn populate(&self, data: &BoxedAny, cmd: &mut EntityCommands);
    fn on_editor(
        &self,
        data: &mut BoxedAny,
        entity: Entity,
        ui: &mut egui::Ui,
        commands: &mut Commands,
    );
    fn make_raw(&self, data: &BoxedAny) -> serde_json::Value;
}

struct YoleckTypeHandlerFor<T>
where
    T: 'static,
    T: YoleckSource,
    T: Serialize,
    for<'de> T: Deserialize<'de>,
{
    _phantom_data: PhantomData<fn() -> T>,
}

impl<T> YoleckTypeHandlerTrait for YoleckTypeHandlerFor<T>
where
    T: 'static,
    T: YoleckSource,
    T: Serialize,
    for<'de> T: Deserialize<'de>,
{
    fn make_concrete(&self, data: serde_json::Value) -> serde_json::Result<BoxedAny> {
        let concrete: T = serde_json::from_value(data)?;
        let dynamic: BoxedAny = Box::new(concrete);
        dynamic.downcast_ref::<T>().unwrap();
        Ok(dynamic)
    }

    fn populate(&self, data: &BoxedAny, cmd: &mut EntityCommands) {
        let concrete = data.downcast_ref::<T>().unwrap();
        concrete.populate(cmd);
    }

    fn on_editor(
        &self,
        data: &mut BoxedAny,
        entity: Entity,
        ui: &mut egui::Ui,
        commands: &mut Commands,
    ) {
        let concrete = data.downcast_mut::<T>().unwrap();
        concrete.edit(ui);
        concrete.populate(&mut commands.entity(entity));
    }

    fn make_raw(&self, data: &BoxedAny) -> serde_json::Value {
        let concrete = data.downcast_ref::<T>().unwrap();
        serde_json::to_value(concrete).unwrap()
    }
}

#[derive(Component, Debug)]
pub struct YoleckRaw {
    pub type_name: String,
    pub data: serde_json::Value,
}

fn yoleck_process_raws(
    raws_query: Query<(Entity, &YoleckRaw)>,
    mut commands: Commands,
    yoleck: Res<YoleckState>,
) {
    for (entity, raw) in raws_query.iter() {
        let mut cmd = commands.entity(entity);
        cmd.remove::<YoleckRaw>();
        let handler = yoleck.type_handlers.get(&raw.type_name).unwrap();
        let concrete = handler.make_concrete(raw.data.clone()).unwrap();
        handler.populate(&concrete, &mut cmd);
        cmd.insert(YoleckManaged {
            name: "".to_owned(),
            type_name: raw.type_name.to_owned(),
            data: concrete,
        });
    }
}
