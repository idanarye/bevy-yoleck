use std::any::{Any, TypeId};
use std::marker::PhantomData;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy::utils::HashMap;
use serde::{Deserialize, Serialize};

use crate::prelude::YoleckEditorState;
use crate::{BoxedAny, YoleckEntityLifecycleStatus, YoleckManaged, YoleckSchedule};

pub trait YoleckComponent:
    Default + Clone + PartialEq + Component + Serialize + for<'a> Deserialize<'a>
{
    const KEY: &'static str;
}

pub struct YoleckEntityType {
    pub name: String,
    pub(crate) components: Vec<Box<dyn YoleckComponentHandler>>,
    #[allow(clippy::type_complexity)]
    pub(crate) on_init:
        Vec<Box<dyn 'static + Sync + Send + Fn(YoleckEditorState, &mut EntityCommands)>>,
}

impl YoleckEntityType {
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            components: Default::default(),
            on_init: Default::default(),
        }
    }

    pub fn with<T: YoleckComponent>(mut self) -> Self {
        self.components
            .push(Box::<YoleckComponentHandlerImpl<T>>::default());
        self
    }

    pub fn insert_on_init<T: Bundle>(
        mut self,
        bundle_maker: impl 'static + Sync + Send + Fn() -> T,
    ) -> Self {
        self.on_init.push(Box::new(move |_, cmd| {
            cmd.insert(bundle_maker());
        }));
        self
    }

    pub fn insert_on_init_during_editor<T: Bundle>(
        mut self,
        bundle_maker: impl 'static + Sync + Send + Fn() -> T,
    ) -> Self {
        self.on_init.push(Box::new(move |editor_state, cmd| {
            if matches!(editor_state, YoleckEditorState::EditorActive) {
                cmd.insert(bundle_maker());
            }
        }));
        self
    }

    pub fn insert_on_init_during_game<T: Bundle>(
        mut self,
        bundle_maker: impl 'static + Sync + Send + Fn() -> T,
    ) -> Self {
        self.on_init.push(Box::new(move |editor_state, cmd| {
            if matches!(editor_state, YoleckEditorState::GameActive) {
                cmd.insert(bundle_maker());
            }
        }));
        self
    }
}

pub(crate) trait YoleckComponentHandler: 'static + Sync + Send {
    fn component_type(&self) -> TypeId;
    fn key(&self) -> &'static str;
    fn init_in_entity(
        &self,
        data: Option<serde_json::Value>,
        cmd: &mut EntityCommands,
        components_data: &mut HashMap<TypeId, BoxedAny>,
    );
    fn build_in_bevy_app(&self, app: &mut App);
    fn serialize(&self, component: &dyn Any) -> serde_json::Value;
}

#[derive(Default)]
struct YoleckComponentHandlerImpl<T: YoleckComponent> {
    _phantom_data: PhantomData<T>,
}

impl<T: YoleckComponent> YoleckComponentHandler for YoleckComponentHandlerImpl<T> {
    fn component_type(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn key(&self) -> &'static str {
        T::KEY
    }

    fn init_in_entity(
        &self,
        data: Option<serde_json::Value>,
        cmd: &mut EntityCommands,
        components_data: &mut HashMap<TypeId, BoxedAny>,
    ) {
        let component: T = if let Some(data) = data {
            match serde_json::from_value(data) {
                Ok(component) => component,
                Err(err) => {
                    error!("Cannot load {:?}: {:?}", T::KEY, err);
                    return;
                }
            }
        } else {
            Default::default()
        };
        components_data.insert(self.component_type(), Box::new(component.clone()));
        cmd.insert(component);
    }

    fn build_in_bevy_app(&self, app: &mut App) {
        if let Some(schedule) = app.get_schedule_mut(YoleckSchedule::UpdateRawDataFromComponents) {
            schedule.add_system(|mut query: Query<(&mut YoleckManaged, &mut T)>| {
                for (mut yoleck_managed, component) in query.iter_mut() {
                    let yoleck_managed = yoleck_managed.as_mut();
                    match yoleck_managed.components_data.entry(TypeId::of::<T>()) {
                        bevy::utils::hashbrown::hash_map::Entry::Vacant(entry) => {
                            yoleck_managed.lifecycle_status =
                                YoleckEntityLifecycleStatus::JustChanged;
                            entry.insert(Box::<T>::new(component.clone()));
                        }
                        bevy::utils::hashbrown::hash_map::Entry::Occupied(mut entry) => {
                            let existing: &T = entry
                                .get()
                                .downcast_ref()
                                .expect("Component data is of wrong type");
                            if existing != component.as_ref() {
                                yoleck_managed.lifecycle_status =
                                    YoleckEntityLifecycleStatus::JustChanged;
                                entry.insert(Box::<T>::new(component.as_ref().clone()));
                            }
                        }
                    }
                }
            });
        }
    }

    fn serialize(&self, component: &dyn Any) -> serde_json::Value {
        let concrete = component
            .downcast_ref::<T>()
            .expect("Serialize must be called with the correct type");
        serde_json::to_value(concrete).expect("Component must always be serializable")
    }
}
