use std::any::TypeId;
use std::marker::PhantomData;

use bevy::ecs::system::{EntityCommands, SystemParam};
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_egui::egui;

use crate::{BoxedArc, YoleckManaged};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum YoleckEditorState {
    EditorActive,
    GameActive,
}

#[derive(Clone, Copy)]
pub(crate) enum PopulateReason {
    EditorInit,
    EditorUpdate,
    RealGame,
}

pub struct YoleckPopulateContext<'a> {
    pub(crate) reason: PopulateReason,
    // I may add stuff that need 'a later, and I don't want to change the signature
    pub(crate) _phantom_data: PhantomData<&'a ()>,
}

impl<'a> YoleckPopulateContext<'a> {
    pub fn is_in_editor(&self) -> bool {
        match self.reason {
            PopulateReason::EditorInit => true,
            PopulateReason::EditorUpdate => true,
            PopulateReason::RealGame => false,
        }
    }

    pub fn is_first_time(&self) -> bool {
        match self.reason {
            PopulateReason::EditorInit => true,
            PopulateReason::EditorUpdate => false,
            PopulateReason::RealGame => true,
        }
    }
}

pub struct YoleckEditContext<'a> {
    pub(crate) passed: &'a HashMap<TypeId, BoxedArc>,
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

pub struct YoleckUiForEditSystem(pub egui::Ui);

#[derive(SystemParam)]
pub struct YoleckEdit<'w, 's, T: 'static> {
    #[allow(dead_code)]
    query: Query<'w, 's, &'static mut YoleckManaged>,
    #[allow(dead_code)]
    context: Res<'w, YoleckUserSystemContext>,
    pub ui: ResMut<'w, YoleckUiForEditSystem>,
    #[system_param(ignore)]
    _phantom_data: PhantomData<fn() -> T>,
}

impl<'w, 's, T: 'static> YoleckEdit<'w, 's, T> {
    pub fn edit(&mut self, mut dlg: impl FnMut(&YoleckEditContext, &mut T, &mut egui::Ui)) {
        match &*self.context {
            YoleckUserSystemContext::Nope
            | YoleckUserSystemContext::PopulateEdited(_)
            | YoleckUserSystemContext::PopulateInitiated { .. } => {
                panic!("Wrong state");
            }
            YoleckUserSystemContext::Edit { entity, passed } => {
                let edit_context = YoleckEditContext { passed };
                let mut yoleck_managed = self
                    .query
                    .get_mut(*entity)
                    .expect("Edited entity does not exist");
                let data = yoleck_managed
                    .data
                    .downcast_mut::<T>()
                    .expect("Edited data is of wrong type");
                dlg(&edit_context, data, &mut self.ui.0);
            }
        }
    }
}

#[derive(SystemParam)]
pub struct YoleckPopulate<'w, 's, T: 'static> {
    query: Query<'w, 's, &'static mut YoleckManaged>,
    context: Res<'w, YoleckUserSystemContext>,
    commands: Commands<'w, 's>,
    #[system_param(ignore)]
    _phantom_data: PhantomData<fn() -> T>,
}

impl<'w, 's, T: 'static> YoleckPopulate<'w, 's, T> {
    pub fn populate(
        &mut self,
        mut dlg: impl FnMut(&YoleckPopulateContext, &mut T, EntityCommands),
    ) {
        match &*self.context {
            YoleckUserSystemContext::Nope | YoleckUserSystemContext::Edit { .. } => {
                panic!("Wrong state");
            }
            YoleckUserSystemContext::PopulateEdited(entity) => {
                let populate_context = YoleckPopulateContext {
                    reason: PopulateReason::EditorUpdate,
                    _phantom_data: Default::default(),
                };
                let mut yoleck_managed = self
                    .query
                    .get_mut(*entity)
                    .expect("Edited entity does not exist");
                let data = yoleck_managed
                    .data
                    .downcast_mut::<T>()
                    .expect("Edited data is of wrong type");
                dlg(&populate_context, data, self.commands.entity(*entity));
            }
            YoleckUserSystemContext::PopulateInitiated {
                is_in_editor,
                entities,
            } => {
                let populate_context = YoleckPopulateContext {
                    reason: if *is_in_editor {
                        PopulateReason::EditorInit
                    } else {
                        PopulateReason::RealGame
                    },
                    _phantom_data: Default::default(),
                };
                for entity in entities {
                    let mut yoleck_managed = self
                        .query
                        .get_mut(*entity)
                        .expect("Edited entity does not exist");
                    let data = yoleck_managed
                        .data
                        .downcast_mut::<T>()
                        .expect("Edited data is of wrong type");
                    dlg(&populate_context, data, self.commands.entity(*entity));
                }
            }
        }
    }
}

pub enum YoleckUserSystemContext {
    Nope,
    Edit {
        entity: Entity,
        passed: HashMap<TypeId, BoxedArc>,
    },
    PopulateEdited(Entity),
    PopulateInitiated {
        is_in_editor: bool,
        entities: Vec<Entity>,
    },
}

impl YoleckUserSystemContext {
    pub(crate) fn get_edit_entity(&self) -> Entity {
        if let Self::Edit { entity, .. } = self {
            *entity
        } else {
            panic!("Wrong state");
        }
    }
}
