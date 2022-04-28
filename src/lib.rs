mod mouse_actions_2d;

use std::any::{Any, TypeId};
use std::marker::PhantomData;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy::utils::{HashMap, HashSet};
use bevy_egui::{egui, EguiContext};
use serde::{Deserialize, Serialize};

pub use mouse_actions_2d::YoleckSelectable;

pub struct YoleckPlugin;

impl Plugin for YoleckPlugin {
    fn build(&self, app: &mut App) {
        app.add_state(YoleckEditorState::EditorActive);
        app.insert_resource(YoleckState {
            entity_being_edited: None,
            type_handler_names: Default::default(),
            type_handlers: Default::default(),
        });
        app.add_event::<YoleckDirective>();
        app.add_system_set(
            SystemSet::on_update(YoleckEditorState::EditorActive).with_system(yoleck_editor),
        );
        app.add_system(yoleck_process_raw_entries);
        app.add_plugin(mouse_actions_2d::YoleckMouseActions2dPlugin);
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum YoleckEditorState {
    EditorActive,
}

pub enum YoleckDirectiveInner {
    PassToEntity(Entity, TypeId, BoxedAny),
}

pub struct YoleckDirective(YoleckDirectiveInner);

impl YoleckDirective {
    pub fn pass_to_entity<T: 'static + Send + Sync>(entity: Entity, data: T) -> Self {
        Self(YoleckDirectiveInner::PassToEntity(
            entity,
            TypeId::of::<T>(),
            Box::new(data),
        ))
    }
}

pub struct YoleckEditContext<'a> {
    passed: &'a HashMap<TypeId, &'a BoxedAny>,
}

impl YoleckEditContext<'_> {
    pub fn get_passed_data<T: 'static>(&self) -> Option<&T> {
        if let Some(dynamic) = self.passed.get(&TypeId::of::<T>()) {
            dynamic.downcast_ref()
        } else {
            None
        }
    }
}

pub trait YoleckSource: Send + Sync {
    fn populate(&self, cmd: &mut EntityCommands);
    fn edit(&mut self, ui: &mut egui::Ui, ctx: &YoleckEditContext);
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
    type_handler_names: Vec<String>,
    type_handlers: HashMap<String, Box<dyn YoleckTypeHandlerTrait>>,
}

impl YoleckState {
    pub fn add_handler<T: 'static>(&mut self, name: String)
    where
        T: YoleckSource,
        T: Serialize,
        for<'de> T: Deserialize<'de>,
    {
        self.type_handler_names.push(name.clone());
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
    mut filter_types: Local<HashSet<String>>,
    mut directives_reader: EventReader<YoleckDirective>,
) {
    let mut data_passed_to_entities: HashMap<Entity, HashMap<TypeId, &BoxedAny>> =
        Default::default();
    let dummy_data_passed_to_entity = HashMap::<TypeId, &BoxedAny>::new();
    for directive in directives_reader.iter() {
        match &directive.0 {
            YoleckDirectiveInner::PassToEntity(entity, type_id, data) => {
                data_passed_to_entities
                    .entry(*entity)
                    .or_default()
                    .insert(*type_id, data);
            }
        }
    }

    egui::Window::new("Level Editor").show(egui_context.ctx_mut(), |ui| {
        let yoleck = yoleck.as_mut();

        if ui.button("Save Level").clicked() {
            for (entity, yoleck_managed) in yoleck_managed_query.iter() {
                let handler = yoleck.type_handlers.get(&yoleck_managed.type_name).unwrap();
                println!(
                    "{:?}: {}",
                    entity,
                    serde_json::to_string(&YoleckRawEntry {
                        header: YoleckEntryHeader {
                            type_name: yoleck_managed.type_name.clone(),
                            name: yoleck_managed.name.clone(),
                        },
                        data: handler.make_raw(&yoleck_managed.data),
                    })
                    .unwrap()
                );
            }
        }

        egui::CollapsingHeader::new("Types")
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new("level editor types table").show(ui, |ui| {
                    for type_name in yoleck.type_handler_names.iter() {
                        let mut should_show = filter_types.contains(type_name);
                        if ui.checkbox(&mut should_show, type_name).changed() {
                            if should_show {
                                filter_types.insert(type_name.clone());
                            } else {
                                filter_types.remove(type_name);
                            }
                        }
                        if ui.button("New").clicked() {
                            let mut cmd = commands.spawn();
                            cmd.insert(YoleckRawEntry {
                                header: YoleckEntryHeader {
                                    type_name: type_name.clone(),
                                    name: "".to_owned(),
                                },
                                data: serde_json::Value::Object(Default::default()),
                            });
                            yoleck.entity_being_edited = Some(cmd.id());
                        }
                        ui.end_row();
                    }
                });
            });

        egui::ScrollArea::vertical()
            .max_height(128.0)
            .show(ui, |ui| {
                for (entity, mut yoleck_managed) in yoleck_managed_query.iter_mut() {
                    if !filter_types.is_empty() && !filter_types.contains(&yoleck_managed.type_name)
                    {
                        continue;
                    }
                    let is_selected = yoleck.entity_being_edited == Some(entity);
                    let header = egui::CollapsingHeader::new(if yoleck_managed.name.is_empty() {
                        format!("{} {:?}", yoleck_managed.type_name, entity)
                    } else {
                        format!(
                            "{} ({} {:?})",
                            yoleck_managed.name, yoleck_managed.type_name, entity
                        )
                    });
                    let header = header.selectable(true).selected(is_selected);
                    let header = header.open(Some(is_selected));
                    let resp = header.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut yoleck_managed.name);
                            if ui.button("Delete").clicked() {}
                        });
                        let handler = yoleck.type_handlers.get(&yoleck_managed.type_name).unwrap();
                        let edit_context = YoleckEditContext {
                            passed: data_passed_to_entities
                                .get(&entity)
                                .unwrap_or(&dummy_data_passed_to_entity),
                        };
                        handler.on_editor(
                            &mut yoleck_managed.data,
                            entity,
                            ui,
                            &edit_context,
                            &mut commands,
                        );
                    });
                    if resp.header_response.clicked() {
                        if is_selected {
                            yoleck.entity_being_edited = None;
                        } else {
                            yoleck.entity_being_edited = Some(entity);
                        }
                    }
                }
            });
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
        ctx: &YoleckEditContext,
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
        ctx: &YoleckEditContext,
        commands: &mut Commands,
    ) {
        let concrete = data.downcast_mut::<T>().unwrap();
        concrete.edit(ui, ctx);
        concrete.populate(&mut commands.entity(entity));
    }

    fn make_raw(&self, data: &BoxedAny) -> serde_json::Value {
        let concrete = data.downcast_ref::<T>().unwrap();
        serde_json::to_value(concrete).unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct YoleckEntryHeader {
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Component, Debug)]
pub struct YoleckRawEntry {
    pub header: YoleckEntryHeader,
    pub data: serde_json::Value,
}

impl Serialize for YoleckRawEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (&self.header, &self.data).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for YoleckRawEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (header, data): (YoleckEntryHeader, serde_json::Value) =
            Deserialize::deserialize(deserializer)?;
        Ok(Self { header, data })
    }
}

fn yoleck_process_raw_entries(
    raw_entries_query: Query<(Entity, &YoleckRawEntry)>,
    mut commands: Commands,
    yoleck: Res<YoleckState>,
) {
    for (entity, raw_entry) in raw_entries_query.iter() {
        let mut cmd = commands.entity(entity);
        cmd.remove::<YoleckRawEntry>();
        let handler = yoleck
            .type_handlers
            .get(&raw_entry.header.type_name)
            .unwrap();
        let concrete = handler.make_concrete(raw_entry.data.clone()).unwrap();
        handler.populate(&concrete, &mut cmd);
        cmd.insert(YoleckManaged {
            name: raw_entry.header.name.to_owned(),
            type_name: raw_entry.header.type_name.to_owned(),
            data: concrete,
        });
    }
}
