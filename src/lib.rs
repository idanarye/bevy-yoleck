mod mouse_actions_2d;

use std::any::{Any, TypeId};
use std::marker::PhantomData;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy::utils::{HashMap, HashSet};
use bevy_egui::{egui, EguiContext};
use serde::{Deserialize, Serialize};

pub struct YoleckPlugin;

impl Plugin for YoleckPlugin {
    fn build(&self, app: &mut App) {
        app.add_state(YoleckEditorState::EditorActive);
        app.insert_resource(YoleckState {
            entity_being_edited: None,
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
    GameActive,
}

pub enum YoleckDirectiveInner {
    SetSelected(Option<Entity>),
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

    pub fn set_selected(entity: Option<Entity>) -> Self {
        Self(YoleckDirectiveInner::SetSelected(entity))
    }
}

#[derive(Clone, Copy)]
enum PopulateReason {
    EditorInit,
    EditorUpdate,
    RealGame,
}

pub struct YoleckPopulateContext<'a> {
    reason: PopulateReason,
    // I may add stuff that need 'a later, and I don't want to change the signature
    _phantom_data: PhantomData<&'a ()>,
}

impl<'a> YoleckPopulateContext<'a> {
    pub fn is_in_editor(&self) -> bool {
        match self.reason {
            PopulateReason::EditorInit => true,
            PopulateReason::EditorUpdate => true,
            PopulateReason::RealGame => false,
        }
    }

    pub fn is_first_tiome(&self) -> bool {
        match self.reason {
            PopulateReason::EditorInit => true,
            PopulateReason::EditorUpdate => false,
            PopulateReason::RealGame => true,
        }
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
    fn populate(&self, ctx: &YoleckPopulateContext, cmd: &mut EntityCommands);
    fn edit(&mut self, ctx: &YoleckEditContext, ui: &mut egui::Ui);

    fn handler(name: impl ToString) -> Box<dyn YoleckTypeHandlerTrait>
    where
        Self: 'static,
        Self: Serialize,
        for<'de> Self: Deserialize<'de>,
    {
        Box::new(YoleckTypeHandlerFor::<Self> {
            type_name: name.to_string(),
            _phantom_data: Default::default(),
        })
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

#[allow(clippy::too_many_arguments)]
fn yoleck_editor(
    mut egui_context: ResMut<EguiContext>,
    mut yoleck: ResMut<YoleckState>,
    yoleck_type_handlers: Res<YoleckTypeHandlers>,
    mut yoleck_managed_query: Query<(Entity, &mut YoleckManaged)>,
    mut commands: Commands,
    mut filter_custom_name: Local<String>,
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
            YoleckDirectiveInner::SetSelected(entity) => {
                yoleck.entity_being_edited = *entity;
            }
        }
    }

    egui::Window::new("Level Editor").show(egui_context.ctx_mut(), |ui| {
        let yoleck = yoleck.as_mut();

        if ui.button("Save Level").clicked() {
            for (entity, yoleck_managed) in yoleck_managed_query.iter() {
                let handler = yoleck_type_handlers
                    .type_handlers
                    .get(&yoleck_managed.type_name)
                    .unwrap();
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

        let popup_id = ui.make_persistent_id("add_new_entity_popup_id");
        let button_response = ui.button("Add New Entity");
        if button_response.clicked() {
            ui.memory().toggle_popup(popup_id);
        }
        egui::popup_below_widget(ui, popup_id, &button_response, |ui| {
            for type_name in yoleck_type_handlers.type_handler_names.iter() {
                if ui.button(type_name).clicked() {
                    let mut cmd = commands.spawn();
                    cmd.insert(YoleckRawEntry {
                        header: YoleckEntryHeader {
                            type_name: type_name.clone(),
                            name: "".to_owned(),
                        },
                        data: serde_json::Value::Object(Default::default()),
                    });
                    yoleck.entity_being_edited = Some(cmd.id());
                    ui.memory().toggle_popup(popup_id);
                }
            }
        });

        fn format_caption(entity: Entity, yoleck_managed: &YoleckManaged) -> String {
            if yoleck_managed.name.is_empty() {
                format!("{} {:?}", yoleck_managed.type_name, entity)
            } else {
                format!(
                    "{} ({} {:?})",
                    yoleck_managed.name, yoleck_managed.type_name, entity
                )
            }
        }

        egui::CollapsingHeader::new("Select").show(ui, |ui| {
            egui::CollapsingHeader::new("Filter").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("By Name:");
                    ui.text_edit_singleline(&mut *filter_custom_name);
                });
                for type_name in yoleck_type_handlers.type_handler_names.iter() {
                    let mut should_show = filter_types.contains(type_name);
                    if ui.checkbox(&mut should_show, type_name).changed() {
                        if should_show {
                            filter_types.insert(type_name.clone());
                        } else {
                            filter_types.remove(type_name);
                        }
                    }
                }
            });
            for (entity, yoleck_managed) in yoleck_managed_query.iter_mut() {
                if !filter_types.is_empty() && !filter_types.contains(&yoleck_managed.type_name) {
                    continue;
                }
                if !yoleck_managed.name.contains(filter_custom_name.as_str()) {
                    continue;
                }
                let is_selected = yoleck.entity_being_edited == Some(entity);
                if ui
                    .selectable_label(is_selected, format_caption(entity, &yoleck_managed))
                    .clicked()
                {
                    if is_selected {
                        yoleck.entity_being_edited = None;
                    } else {
                        yoleck.entity_being_edited = Some(entity);
                    }
                }
            }
        });

        if let Some((entity, mut yoleck_managed)) = yoleck
            .entity_being_edited
            .and_then(|entity| yoleck_managed_query.get_mut(entity).ok())
        {
            ui.horizontal(|ui| {
                ui.heading(format!(
                    "Editing {}",
                    format_caption(entity, &yoleck_managed)
                ));
                if ui.button("Delete").clicked() {}
            });
            ui.horizontal(|ui| {
                ui.label("Custom Name:");
                ui.text_edit_singleline(&mut yoleck_managed.name);
            });
            let handler = yoleck_type_handlers
                .type_handlers
                .get(&yoleck_managed.type_name)
                .unwrap();
            let edit_ctx = YoleckEditContext {
                passed: data_passed_to_entities
                    .get(&entity)
                    .unwrap_or(&dummy_data_passed_to_entity),
            };
            let populate_ctx = YoleckPopulateContext {
                reason: PopulateReason::EditorUpdate,
                _phantom_data: Default::default(),
            };
            handler.on_editor(
                &mut yoleck_managed.data,
                entity,
                &edit_ctx,
                ui,
                &populate_ctx,
                &mut commands,
            );
        }
    });
}

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

struct YoleckTypeHandlerFor<T>
where
    T: 'static,
    T: YoleckSource,
    T: Serialize,
    for<'de> T: Deserialize<'de>,
{
    type_name: String,
    _phantom_data: PhantomData<fn() -> T>,
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
    yoleck_type_handlers: Res<YoleckTypeHandlers>,
    editor_state: Res<State<YoleckEditorState>>,
) {
    let populate_reason = match editor_state.current() {
        YoleckEditorState::EditorActive => PopulateReason::EditorInit,
        YoleckEditorState::GameActive => PopulateReason::RealGame,
    };
    for (entity, raw_entry) in raw_entries_query.iter() {
        let mut cmd = commands.entity(entity);
        cmd.remove::<YoleckRawEntry>();
        let handler = yoleck_type_handlers
            .type_handlers
            .get(&raw_entry.header.type_name)
            .unwrap();
        let concrete = handler.make_concrete(raw_entry.data.clone()).unwrap();
        let populate_ctx = YoleckPopulateContext {
            reason: populate_reason,
            _phantom_data: Default::default(),
        };
        handler.populate(&concrete, &populate_ctx, &mut cmd);
        cmd.insert(YoleckManaged {
            name: raw_entry.header.name.to_owned(),
            type_name: raw_entry.header.type_name.to_owned(),
            data: concrete,
        });
    }
}
